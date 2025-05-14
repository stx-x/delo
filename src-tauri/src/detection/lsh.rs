use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use rayon::prelude::*;
use bit_vec::BitVec;
use crate::core::types::HashAlgorithm;
use crate::core::utils::hash_utils::hash_string_to_bitvec;

type BucketKey = u64; // 使用u64作为桶的键类型，更高效

/// LSH (局部敏感哈希) 索引
/// 使用多个桶来存储哈希值，相似的哈希值会被分配到相同的桶中
#[derive(Debug)]
pub struct LSHIndex {
    /// 哈希表: 键是桶标识符(u64)，值是哈希索引列表
    /// 使用固定大小的u64代替String作为键，大幅减少内存占用
    buckets: HashMap<BucketKey, Vec<usize>>,
    /// 哈希分割的段数
    bands: usize,
    /// 算法类型
    algorithm: HashAlgorithm,
    /// 每个桶的最大索引数量，防止热点桶
    max_bucket_size: usize,
    /// 全局二进制特征缓存
    binary_cache: HashMap<usize, BitVec>,
}

impl LSHIndex {
    /// 创建新的LSH索引
    pub fn new(algorithm: HashAlgorithm) -> Self {
        // 根据算法类型选择合适的段数
        let bands = match algorithm {
            HashAlgorithm::Exact => 1,    // 精确匹配使用单一段
            HashAlgorithm::ORB => 6,      // ORB算法使用更多的段，增加精度
            HashAlgorithm::Average => 4,  // 均值哈希使用中等数量的段
            HashAlgorithm::Difference => 4, // 差值哈希使用中等数量的段
            HashAlgorithm::Perceptual => 8, // 感知哈希使用更多段以提高准确性
        };
        
        // 预计容量根据算法类型调整
        let estimated_capacity = match algorithm {
            HashAlgorithm::ORB => 10000,   // ORB特征可能会生成更多的桶
            HashAlgorithm::Exact => 500,   // 精确匹配桶最少
            _ => 3000,                     // 其他哈希算法的桶数适中
        };
        
        Self {
            buckets: HashMap::with_capacity(estimated_capacity),
            bands,
            algorithm,
            max_bucket_size: 2000, // 增大限制以提高查全率
            binary_cache: HashMap::new(),
        }
    }
    
    /// 添加哈希值到索引中
    pub fn add(&mut self, hash: &str, index: usize) {
        if hash.is_empty() {
            return; // 跳过空哈希值
        }
        
        // 将哈希字符串转换为二进制表示，以便更高效处理
        let bit_vec = hash_string_to_bitvec(hash);
        
        // 缓存二进制表示，以便在查询时重用
        self.binary_cache.insert(index, bit_vec.clone());
        
        // 根据算法类型生成桶标识符
        let bucket_keys = self.generate_bucket_keys(hash, &bit_vec);
        
        // 限制添加到每个桶的索引数量，避免某些热点桶过大
        for key in bucket_keys {
            let bucket = self.buckets.entry(key).or_insert_with(|| Vec::with_capacity(8));
            
            // 避免完全相同的索引被添加多次
            if !bucket.contains(&index) && bucket.len() < self.max_bucket_size {
                bucket.push(index);
            }
        }
    }
    
    /// 根据哈希值和算法类型生成桶标识符（u64）
    fn generate_bucket_keys(&self, hash: &str, bit_vec: &BitVec) -> Vec<BucketKey> {
        match self.algorithm {
            // 对于ORB算法，使用特殊处理
            HashAlgorithm::ORB => {
                // ORB算法使用分段哈希以及特征聚合
                self.generate_orb_bucket_keys(hash)
            },
            // 对于二进制哈希，直接使用比特切片
            HashAlgorithm::Average | HashAlgorithm::Difference | HashAlgorithm::Perceptual => {
                self.generate_binary_bucket_keys(bit_vec)
            },
            // 对于精确哈希，使用哈希的前8字节作为单一桶键
            HashAlgorithm::Exact => {
                vec![self.hash_to_u64(hash)]
            }
        }
    }
    
    /// 为ORB特征生成桶键
    fn generate_orb_bucket_keys(&self, hash: &str) -> Vec<BucketKey> {
        // ORB特征需要更复杂的处理

        // 针对ORB特征的Base64编码特点
        let is_compressed = hash.len() > 20 && hash.starts_with("eJ");
        
        if is_compressed {
            // 针对压缩特征，使用更高效的编码方式
            if hash.len() <= self.bands * 8 {
                // 哈希太短，使用全部
                vec![self.hash_to_u64(hash)]
            } else {
                // 将特征分为多个块，每个块都可以作为一个桶
                let chunk_size = hash.len() / self.bands;
                (0..self.bands)
                    .map(|i| {
                        let start = i * chunk_size;
                        let end = if i == self.bands - 1 { hash.len() } else { (i + 1) * chunk_size };
                        let chunk = &hash[start..end];
                        self.hash_to_u64(chunk)
                    })
                    .collect()
            }
        } else {
            // 使用传统方式处理非压缩特征
            let signature_len = if hash.len() > 384 { 384 } else { hash.len() };
            let signature = &hash[0..signature_len];
            
            // 将特征分成多个部分，每部分生成一个桶键
            let chunk_size = signature_len / self.bands;
            if chunk_size > 0 {
                (0..self.bands)
                    .map(|i| {
                        let start = i * chunk_size;
                        let end = if i == self.bands - 1 { signature_len } else { (i + 1) * chunk_size };
                        let chunk = &signature[start..end];
                        self.hash_to_u64(chunk)
                    })
                    .collect()
            } else {
                // 哈希太短，使用整个哈希作为一个桶
                vec![self.hash_to_u64(signature)]
            }
        }
    }
    
    /// 为二进制哈希生成桶键
    fn generate_binary_bucket_keys(&self, bit_vec: &BitVec) -> Vec<BucketKey> {
        let len = bit_vec.len();
        let band_size = len / self.bands;
        
        if band_size == 0 {
            // 如果位向量太短，使用整个向量作为一个桶
            return vec![self.bitvec_to_u64(bit_vec)];
        }
        
        // 将位向量分成多个部分，每部分生成一个桶键
        let mut result = Vec::with_capacity(self.bands);
        for i in 0..self.bands {
            let start = i * band_size;
            let end = if i == self.bands - 1 { len } else { (i + 1) * band_size };
            
            // 提取当前段的位向量
            let slice = bit_vec.iter().skip(start).take(end - start);
            
            // 将位向量片段转换为u64
            let mut key: u64 = 0;
            for (j, bit) in slice.enumerate() {
                if bit && j < 64 {
                    key |= 1 << j;
                }
            }
            result.push(key);
        }
        result
    }
    
    /// 将哈希字符串转换为u64桶键
    fn hash_to_u64(&self, s: &str) -> BucketKey {
        // 简单但有效的哈希函数
        let mut h: u64 = 0;
        for b in s.bytes() {
            h = h.wrapping_mul(31).wrapping_add(b as u64);
        }
        h
    }
    
    /// 将位向量转换为u64
    fn bitvec_to_u64(&self, bv: &BitVec) -> BucketKey {
        let mut result: u64 = 0;
        for (i, bit) in bv.iter().enumerate().take(64) {
            if bit {
                result |= 1 << i;
            }
        }
        result
    }
    
    /// 查询与给定哈希值可能相似的所有索引
    pub fn query(&self, hash: &str) -> Vec<usize> {
        if hash.is_empty() {
            return Vec::new(); // 跳过空哈希值
        }
        
        // 将哈希转换为二进制表示
        let bit_vec = hash_string_to_bitvec(hash);
        
        // 生成与添加时相同的桶键
        let bucket_keys = self.generate_bucket_keys(hash, &bit_vec);
        
        // 使用HashSet进行自动去重，提高效率
        let mut candidates = HashSet::with_capacity(100);
        
        // 查询每个桶
        for key in bucket_keys {
            if let Some(indices) = self.buckets.get(&key) {
                candidates.extend(indices);
            }
        }
        
        // 转换为向量并排序以提高缓存局部性
        let mut result: Vec<_> = candidates.into_iter().collect();
        result.sort_unstable();
        result
    }
    
    /// 批量添加哈希值到索引中
    pub fn batch_add(&mut self, hashes: &[String], start_index: usize) {
        // 小批量直接处理
        if hashes.len() <= 500 {
            for (i, hash) in hashes.iter().enumerate() {
                self.add(hash, start_index + i);
            }
            return;
        }
        
        // 大批量并行处理
        // 预先计算所有哈希的二进制表示
        let bit_vectors: Vec<_> = hashes.par_iter()
            .map(|hash| hash_string_to_bitvec(hash))
            .collect();
            
        // 并行生成所有桶键
        let all_keys: Vec<_> = hashes.par_iter()
            .zip(bit_vectors.par_iter())
            .enumerate()
            .map(|(i, (hash, bit_vec))| {
                let idx = start_index + i;
                let keys = self.generate_bucket_keys(hash, bit_vec);
                (idx, keys)
            })
            .collect();
            
        // 更新二进制缓存
        for (i, hash) in hashes.iter().enumerate() {
            let idx = start_index + i;
            let bit_vec = hash_string_to_bitvec(hash);
            self.binary_cache.insert(idx, bit_vec);
        }
        
        // 分批处理以避免内存峰值
        // 使用互斥锁保证线程安全的桶更新
        let buckets_mutex = Arc::new(parking_lot::Mutex::new(&mut self.buckets));
        
        all_keys.chunks(1000).par_bridge().for_each(|chunk| {
            // 创建局部更新批次
            let mut local_updates: HashMap<BucketKey, Vec<usize>> = HashMap::new();
            
            // 在本地收集桶更新
            for (idx, keys) in chunk {
                for key in keys {
                    local_updates.entry(*key)
                        .or_insert_with(Vec::new)
                        .push(*idx);
                }
            }
            
            // 获取锁并应用批量更新
            let mut guard = buckets_mutex.lock();
            for (key, indices) in local_updates {
                let bucket = guard.entry(key).or_insert_with(Vec::new);
                for idx in indices {
                    if !bucket.contains(&idx) && bucket.len() < self.max_bucket_size {
                        bucket.push(idx);
                    }
                }
            }
        });
    }
    
    /// 清空索引
    pub fn clear(&mut self) {
        self.buckets.clear();
    }
    
    /// 获取索引中的哈希数量（去重）
    pub fn len(&self) -> usize {
        // 使用二进制缓存的大小作为索引大小，更准确更高效
        self.binary_cache.len()
    }
    
    /// 检查索引是否为空
    pub fn is_empty(&self) -> bool {
        self.buckets.is_empty()
    }
}

/// 并行计算候选匹配对
/// 对于大量哈希值，使用LSH并行计算可能的相似对
pub fn compute_candidate_pairs(hashes: &[String], algorithm: HashAlgorithm) -> Vec<(usize, usize)> {
    if hashes.len() <= 1 {
        return Vec::new();
    }
    
    // 针对不同大小的数据集使用不同策略
    if hashes.len() <= 5000 {
        // 小型数据集：单个LSH索引处理全部数据
        compute_pairs_small_dataset(hashes, algorithm)
    } else {
        // 大型数据集：分片策略
        compute_pairs_large_dataset(hashes, algorithm)
    }
}

/// 处理小型数据集的候选对计算
fn compute_pairs_small_dataset(hashes: &[String], algorithm: HashAlgorithm) -> Vec<(usize, usize)> {
    // 创建单个LSH索引
    let mut lsh = LSHIndex::new(algorithm);
    
    // 批量添加所有哈希值
    lsh.batch_add(hashes, 0);
    
    // 并行查询生成候选对
    let pairs: HashSet<(usize, usize)> = (0..hashes.len()).into_par_iter()
        .flat_map(|i| {
            let hash = &hashes[i];
            let candidates = lsh.query(hash);
            
            // 只保留i < j的对，避免重复
            candidates.into_iter()
                .filter_map(move |j| if j > i { Some((i, j)) } else { None })
                .collect::<Vec<_>>()
        })
        .collect();
    
    // 转换为向量
    pairs.into_iter().collect()
}

/// 处理大型数据集的候选对计算
fn compute_pairs_large_dataset(hashes: &[String], algorithm: HashAlgorithm) -> Vec<(usize, usize)> {
    // 优化的批处理大小，根据算法类型调整
    let batch_size = match algorithm {
        HashAlgorithm::ORB => 2000,     // ORB特征处理成本较高
        HashAlgorithm::Exact => 10000,  // 精确匹配处理成本较低
        _ => 5000,                      // 其他算法适中
    };
    
    // 计算批次数
    let batch_count = (hashes.len() + batch_size - 1) / batch_size;
    
    // 使用共享结果收集器
    let result_pairs = Arc::new(parking_lot::Mutex::new(HashSet::with_capacity(hashes.len() / 2)));
    
    // 第一阶段：处理内部批次
    (0..batch_count).into_par_iter().for_each(|batch_idx| {
        let start = batch_idx * batch_size;
        let end = (start + batch_size).min(hashes.len());
        let batch = &hashes[start..end];
        
        // 为当前批次创建LSH索引
        let mut lsh = LSHIndex::new(algorithm);
        for (i, hash) in batch.iter().enumerate() {
            lsh.add(hash, i);
        }
        
        // 计算批次内部匹配
        let mut local_pairs = HashSet::new();
        
        for i in 0..batch.len() {
            let hash = &batch[i];
            let candidates = lsh.query(hash);
            
            for &j in &candidates {
                if j > i {
                    local_pairs.insert((i + start, j + start));
                }
            }
        }
        
        // 合并到全局结果
        let mut guard = result_pairs.lock();
        guard.extend(local_pairs);
    });
    
    // 第二阶段：处理批次之间的匹配，避免重复构建索引
    let all_lsh_indices: Vec<LSHIndex> = (0..batch_count).into_par_iter()
        .map(|batch_idx| {
            let start = batch_idx * batch_size;
            let end = (start + batch_size).min(hashes.len());
            let batch = &hashes[start..end];
            
            let mut lsh = LSHIndex::new(algorithm);
            for (i, hash) in batch.iter().enumerate() {
                lsh.add(hash, i);
            }
            lsh
        })
        .collect();
    
    // 跨批次匹配，避免重复比较
    (0..batch_count).into_par_iter().for_each(|i| {
        // 只需要比较i < j的批次，避免重复
        for j in (i + 1)..batch_count {
            let i_start = i * batch_size;
            let j_start = j * batch_size;
            
            // 获取各自的批次
            let _i_batch = &hashes[i_start..(i_start + batch_size).min(hashes.len())];
            let j_batch = &hashes[j_start..(j_start + batch_size).min(hashes.len())];
            
            // 获取各自的LSH索引
            let i_lsh = &all_lsh_indices[i];
            
            // 使用批次j的每个哈希查询批次i的LSH索引
            let cross_pairs: Vec<(usize, usize)> = j_batch.par_iter()
                .enumerate()
                .flat_map(|(j_idx, hash)| {
                    let i_candidates = i_lsh.query(hash);
                    i_candidates.into_iter()
                        .map(move |i_idx| (i_idx + i_start, j_idx + j_start))
                        .collect::<Vec<_>>()
                })
                .collect();
                
            // 合并到全局结果
            let mut guard = result_pairs.lock();
            guard.extend(cross_pairs);
        }
    });
    
    // 转换结果为Vec并排序
    let mut result: Vec<_> = Arc::try_unwrap(result_pairs)
        .expect("Failed to unwrap Arc")
        .into_inner()
        .into_iter()
        .collect();
        
    result.sort_unstable();
    result
}