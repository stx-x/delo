use std::collections::{HashMap, HashSet};
use rayon::prelude::*;
use crate::core::types::HashAlgorithm;
use crate::core::utils::hash_utils::split_hash_for_lsh;

/// LSH (局部敏感哈希) 索引
/// 使用多个桶来存储哈希值，相似的哈希值会被分配到相同的桶中
#[derive(Debug)]
pub struct LSHIndex {
    /// 哈希表: 键是桶标识符，值是哈希索引列表
    buckets: HashMap<String, Vec<usize>>,
    /// 哈希分割的段数
    bands: usize,
    /// 算法类型
    algorithm: HashAlgorithm,
    /// 每个桶的最大索引数量，防止热点桶
    max_bucket_size: usize,
}

impl LSHIndex {
    /// 创建新的LSH索引
    pub fn new(algorithm: HashAlgorithm) -> Self {
        // 根据算法类型选择合适的段数和桶大小
        let (bands, max_bucket_size) = match algorithm {
            HashAlgorithm::Exact => (1, 1000),    // 精确匹配使用较小的桶
            HashAlgorithm::ORB => (8, 3000),      // ORB需要更大的桶来处理特征匹配
            HashAlgorithm::Average => (4, 2000),   // 均值哈希使用中等大小
            HashAlgorithm::Difference => (4, 2000), // 差值哈希使用中等大小
            HashAlgorithm::Perceptual => (6, 2000), // 感知哈希使用较多的段
        };
        
        Self {
            buckets: HashMap::with_capacity(2000),
            bands,
            algorithm,
            max_bucket_size,
        }
    }
    
    /// 添加哈希值到索引中
    pub fn add(&mut self, hash: &str, index: usize) {
        if hash.is_empty() {
            return; // 跳过空哈希值
        }
        
        let bands = match self.algorithm {
            // 对于ORB算法的特征字符串，通常会很长，使用特殊处理策略
            HashAlgorithm::ORB => {
                // 提取固定数量的字符以创建签名
                let signature_len = if hash.len() > 256 { 256 } else { hash.len() };
                let signature = &hash[0..signature_len];
                
                // 对于ORB，简单地将所有bands个字符块作为签名
                let band_size = signature_len / self.bands;
                if band_size > 0 {
                    split_hash_for_lsh(signature, self.bands)
                } else {
                    // 如果哈希太短，则使用整个哈希值
                    vec![signature.to_string()]
                }
            },
            // 对于其他哈希算法，采用标准分段方式
            _ => split_hash_for_lsh(hash, self.bands),
        };
        
        // 限制添加到每个桶的索引数量，避免某些热点桶过大
        for band in bands {
            let bucket = self.buckets.entry(band).or_insert_with(Vec::new);
            if bucket.len() < self.max_bucket_size {
                bucket.push(index);
            }
        }
    }
    
    /// 查询与给定哈希值可能相似的所有索引
    pub fn query(&self, hash: &str) -> Vec<usize> {
        if hash.is_empty() {
            return Vec::new();
        }
        
        let bands = self.get_hash_bands(hash);
        let mut candidates = HashSet::with_capacity(bands.len() * self.max_bucket_size / 4);
        
        // 收集所有候选索引
        for band in bands {
            if let Some(indices) = self.buckets.get(&band) {
                candidates.extend(indices.iter().copied());
            }
        }
        
        candidates.into_iter().collect()
    }
    
    /// 获取哈希值的LSH段
    fn get_hash_bands(&self, hash: &str) -> Vec<String> {
        if hash.is_empty() {
            return Vec::new();
        }
        
        match self.algorithm {
            HashAlgorithm::ORB => {
                // 对ORB特征使用特殊的分段策略
                let signature_len = if hash.len() > 256 { 256 } else { hash.len() };
                let signature = &hash[0..signature_len];
                
                let band_size = signature_len / self.bands;
                if band_size > 0 {
                    split_hash_for_lsh(signature, self.bands)
                } else {
                    vec![signature.to_string()]
                }
            },
            _ => split_hash_for_lsh(hash, self.bands),
        }
    }
    
    /// 批量添加哈希值到索引中
    pub fn batch_add(&mut self, hashes: &[String], start_index: usize) {
        if hashes.is_empty() {
            return;
        }

        // 优化的批量处理策略
        let batch_size = if hashes.len() > 1000 {
            (hashes.len() / rayon::current_num_threads()).max(500)
        } else {
            hashes.len()
        };

        // 并行处理批次
        let partial_indices: Vec<_> = hashes
            .par_chunks(batch_size)
            .enumerate()
            .map(|(batch_idx, batch_hashes)| {
                let mut local_buckets = HashMap::with_capacity(batch_hashes.len() / 2);
                
                for (i, hash) in batch_hashes.iter().enumerate() {
                    if !hash.is_empty() {
                        let idx = start_index + batch_idx * batch_size + i;
                        let bands = self.get_hash_bands(hash);
                        
                        for band in bands {
                            local_buckets.entry(band)
                                .or_insert_with(Vec::new)
                                .push(idx);
                        }
                    }
                }
                
                local_buckets
            })
            .collect();

        // 优化的合并过程
        let mut new_buckets = HashMap::with_capacity(self.buckets.len() + hashes.len() / 2);
        
        // 首先合并现有的桶
        for (band, indices) in self.buckets.drain() {
            new_buckets.insert(band, indices);
        }
        
        // 合并新的批次结果
        for local_buckets in partial_indices {
            for (band, mut indices) in local_buckets {
                let bucket = new_buckets.entry(band).or_insert_with(Vec::new);
                bucket.append(&mut indices);
                
                // 动态调整桶大小
                if bucket.len() > self.max_bucket_size {
                    bucket.sort_unstable();
                    bucket.dedup();
                    if bucket.len() > self.max_bucket_size {
                        // 保留最新的索引
                        let start = bucket.len() - self.max_bucket_size;
                        bucket.copy_within(start.., 0);
                        bucket.truncate(self.max_bucket_size);
                    }
                }
            }
        }
        
        self.buckets = new_buckets;
    }
    
    /// 清空索引
    pub fn clear(&mut self) {
        self.buckets.clear();
    }
    
    /// 获取索引中的哈希数量（去重）
    pub fn len(&self) -> usize {
        // 计算所有索引的并集大小，防止重复计数
        let mut all_indices = HashSet::<usize>::new();
        for indices in self.buckets.values() {
            all_indices.extend(indices);
        }
        all_indices.len()
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
    
    // 使用更有效的分批处理方式
    const BATCH_SIZE: usize = 10000;
    
    if hashes.len() > BATCH_SIZE {
        // 对于超大规模输入，分批处理以降低内存占用
        let batch_count = (hashes.len() + BATCH_SIZE - 1) / BATCH_SIZE;
        let mut all_pairs = Vec::new();
        
        // 处理批次内部的匹配
        for batch_idx in 0..batch_count {
            let start = batch_idx * BATCH_SIZE;
            let end = (start + BATCH_SIZE).min(hashes.len());
            let batch = &hashes[start..end];
            
            // 计算批次内部的匹配对
            let mut lsh = LSHIndex::new(algorithm);
            for (i, hash) in batch.iter().enumerate() {
                lsh.add(hash, i);
            }
            
            // 并行查询每个哈希值
            let batch_pairs: Vec<(usize, usize)> = batch.par_iter()
                .enumerate()
                .flat_map(|(i, hash)| {
                    let candidate_indices = lsh.query(hash);
                    candidate_indices.into_iter()
                        .filter_map(move |j| {
                            // 避免自匹配和重复匹配
                            if j > i {
                                Some((i + start, j + start))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .collect();
            
            all_pairs.extend(batch_pairs);
            
            // 处理不同批次之间的匹配
            if batch_idx > 0 {
                for prev_batch_idx in 0..batch_idx {
                    let prev_start = prev_batch_idx * BATCH_SIZE;
                    let prev_end = (prev_start + BATCH_SIZE).min(hashes.len());
                    let prev_batch = &hashes[prev_start..prev_end];
                    
                    // 创建新的LSH索引用于跨批次匹配
                    let mut cross_lsh = LSHIndex::new(algorithm);
                    for (i, hash) in prev_batch.iter().enumerate() {
                        cross_lsh.add(hash, i);
                    }
                    
                    // 当前批次的每个哈希查询前面批次的索引
                    let cross_pairs: Vec<(usize, usize)> = batch.par_iter()
                        .enumerate()
                        .flat_map(|(i, hash)| {
                            let prev_indices = cross_lsh.query(hash);
                            prev_indices.into_iter()
                                .map(move |j| (i + start, j + prev_start))
                                .collect::<Vec<_>>()
                        })
                        .collect();
                    
                    all_pairs.extend(cross_pairs);
                }
            }
        }
        
        // 去重
        let mut unique_pairs = HashSet::with_capacity(all_pairs.len());
        for pair in all_pairs {
            unique_pairs.insert(pair);
        }
        
        unique_pairs.into_iter().collect()
    } else {
        // 对于小规模数据，使用原始方法
        // 创建LSH索引
        let mut lsh = LSHIndex::new(algorithm);
        
        // 添加所有哈希值到索引
        for (i, hash) in hashes.iter().enumerate() {
            lsh.add(hash, i);
        }
        
        // 并行查询所有候选对
        let pairs: HashSet<(usize, usize)> = hashes.par_iter()
            .enumerate()
            .flat_map(|(i, hash)| {
                let candidate_indices = lsh.query(hash);
                candidate_indices.into_iter()
                    .filter_map(move |j| {
                        // 避免自匹配和重复匹配
                        if j > i {
                            Some((i, j))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        
        pairs.into_iter().collect()
    }
}