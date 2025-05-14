use std::collections::HashMap;
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
}

impl LSHIndex {
    /// 创建新的LSH索引
    pub fn new(algorithm: HashAlgorithm) -> Self {
        // 根据算法类型选择合适的段数
        let bands = match algorithm {
            HashAlgorithm::Exact => 1,    // 精确匹配使用单一段
            HashAlgorithm::ORB => 2,      // ORB算法使用较少的段
            HashAlgorithm::Average => 4,  // 均值哈希使用中等数量的段
            HashAlgorithm::Difference => 4, // 差值哈希使用中等数量的段
            HashAlgorithm::Perceptual => 4, // 感知哈希使用中等数量的段
        };
        
        Self {
            buckets: HashMap::new(),
            bands,
            algorithm,
        }
    }
    
    /// 添加哈希值到索引中
    pub fn add(&mut self, hash: &str, index: usize) {
        // 对于ORB算法的特征字符串，通常会很长，需要特殊处理
        if self.algorithm == HashAlgorithm::ORB && hash.len() > 100 {
            // 对于ORB，只使用前缀作为LSH索引
            let prefix = &hash[0..hash.len().min(100)];
            let bands = split_hash_for_lsh(prefix, self.bands);
            
            for band in bands {
                self.buckets.entry(band).or_insert_with(Vec::new).push(index);
            }
        } else {
            // 对于其他哈希算法，正常分段
            let bands = split_hash_for_lsh(hash, self.bands);
            
            for band in bands {
                self.buckets.entry(band).or_insert_with(Vec::new).push(index);
            }
        }
    }
    
    /// 查询与给定哈希值可能相似的所有索引
    pub fn query(&self, hash: &str) -> Vec<usize> {
        let mut candidates = Vec::new();
        
        // 对ORB特征特殊处理
        let query_hash = if self.algorithm == HashAlgorithm::ORB && hash.len() > 100 {
            &hash[0..hash.len().min(100)]
        } else {
            hash
        };
        
        // 将哈希分段后查询每个桶
        let bands = split_hash_for_lsh(query_hash, self.bands);
        
        for band in bands {
            if let Some(indices) = self.buckets.get(&band) {
                candidates.extend(indices);
            }
        }
        
        // 去重
        candidates.sort_unstable();
        candidates.dedup();
        
        candidates
    }
    
    /// 批量添加哈希值到索引中
    pub fn batch_add(&mut self, hashes: &[String], start_index: usize) {
        for (i, hash) in hashes.iter().enumerate() {
            self.add(hash, start_index + i);
        }
    }
    
    /// 清空索引
    pub fn clear(&mut self) {
        self.buckets.clear();
    }
    
    /// 获取索引中的哈希数量
    pub fn len(&self) -> usize {
        self.buckets.values().map(|v| v.len()).sum()
    }
    
    /// 检查索引是否为空
    pub fn is_empty(&self) -> bool {
        self.buckets.is_empty() || self.len() == 0
    }
}

/// 并行计算候选匹配对
/// 对于大量哈希值，使用LSH并行计算可能的相似对
pub fn compute_candidate_pairs(hashes: &[String], algorithm: HashAlgorithm) -> Vec<(usize, usize)> {
    if hashes.len() <= 1 {
        return Vec::new();
    }
    
    // 创建LSH索引
    let mut lsh = LSHIndex::new(algorithm);
    
    // 添加所有哈希值到索引
    for (i, hash) in hashes.iter().enumerate() {
        lsh.add(hash, i);
    }
    
    // 并行查询每个哈希值的候选匹配
    let candidates: Vec<Vec<(usize, usize)>> = hashes.par_iter().enumerate().map(|(i, hash)| {
        let mut pairs = Vec::new();
        let candidate_indices = lsh.query(hash);
        
        for &j in &candidate_indices {
            // 避免自匹配和重复匹配
            if j > i {
                pairs.push((i, j));
            }
        }
        
        pairs
    }).collect();
    
    // 合并所有候选对
    let mut all_pairs = Vec::new();
    for pairs in candidates {
        all_pairs.extend(pairs);
    }
    
    // 去重
    all_pairs.sort_unstable();
    all_pairs.dedup();
    
    all_pairs
}