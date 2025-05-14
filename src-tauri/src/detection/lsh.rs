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
        // 根据算法类型选择合适的段数
        let bands = match algorithm {
            HashAlgorithm::Exact => 1,    // 精确匹配使用单一段
            HashAlgorithm::ORB => 4,      // ORB算法使用较多的段以提高准确性
            HashAlgorithm::Average => 4,  // 均值哈希使用中等数量的段
            HashAlgorithm::Difference => 4, // 差值哈希使用中等数量的段
            HashAlgorithm::Perceptual => 6, // 感知哈希使用更多段以提高准确性
        };
        
        Self {
            buckets: HashMap::with_capacity(1000), // 预分配空间减少rehash
            bands,
            algorithm,
            max_bucket_size: 1000, // 限制每个桶的最大大小
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
            return Vec::new(); // 跳过空哈希值
        }
        
        // 对不同算法使用专门的查询策略
        let bands = match self.algorithm {
            HashAlgorithm::ORB => {
                // 与add函数保持一致的处理方式
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
        };
        
        // 使用HashSet进行自动去重，提高效率
        let mut candidates = HashSet::with_capacity(100);
        
        // 将哈希分段后查询每个桶
        for band in bands {
            if let Some(indices) = self.buckets.get(&band) {
                candidates.extend(indices);
            }
        }
        
        // 转换为向量
        candidates.into_iter().collect()
    }
    
    /// 批量添加哈希值到索引中
    pub fn batch_add(&mut self, hashes: &[String], start_index: usize) {
        // 并行处理大批量添加操作，性能更好
        if hashes.len() > 1000 {
            // 创建分批作业
            let batch_size = 500;
            let batches: Vec<_> = hashes.chunks(batch_size).collect();
            
            // 为每个批次创建一个局部LSH索引
            let partial_indices: Vec<_> = batches
                .into_par_iter()
                .enumerate()
                .map(|(batch_idx, batch_hashes)| {
                    let mut local_lsh = LSHIndex::new(self.algorithm);
                    for (i, hash) in batch_hashes.iter().enumerate() {
                        let idx = start_index + batch_idx * batch_size + i;
                        local_lsh.add(hash, idx);
                    }
                    local_lsh
                })
                .collect();
            
            // 合并所有局部索引
            for local_lsh in partial_indices {
                for (band, indices) in local_lsh.buckets {
                    let bucket = self.buckets.entry(band).or_insert_with(Vec::new);
                    for idx in indices {
                        if bucket.len() < self.max_bucket_size {
                            bucket.push(idx);
                        }
                    }
                }
            }
        } else {
            // 对于小批量，直接串行处理
            for (i, hash) in hashes.iter().enumerate() {
                self.add(hash, start_index + i);
            }
        }
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