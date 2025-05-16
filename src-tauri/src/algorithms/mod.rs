pub mod exact_hash;
pub mod average_hash;
pub mod difference_hash; 
pub mod perceptual_hash;
pub mod orb;
// pub mod orb_hash;

use std::path::Path;
use crate::core::types::{HashAlgorithm, HashResult};

/// 计算图像哈希的统一接口
pub fn calculate_hash(path: &Path, algorithm: HashAlgorithm) -> Result<HashResult, String> {
    match algorithm {
        HashAlgorithm::Exact => exact_hash::calculate_exact_hash(path),
        HashAlgorithm::Average => average_hash::calculate_average_hash(path),
        HashAlgorithm::Difference => difference_hash::calculate_difference_hash(path),
        HashAlgorithm::Perceptual => perceptual_hash::calculate_perceptual_hash(path),
        HashAlgorithm::ORB => orb::calculate_orb_features(path),
        // HashAlgorithm::ORB => orb_hash::calculate_orb_hash(path),
    }
}

/// 计算两个哈希值之间的相似度 (0-100)
pub fn calculate_similarity(hash1: &str, hash2: &str, algorithm: HashAlgorithm) -> f32 {
    match algorithm {
        HashAlgorithm::Exact => {
            // 精确哈希: 相同为100%，不同为0%
            if hash1 == hash2 { 100.0 } else { 0.0 }
        },
        HashAlgorithm::Average |
        HashAlgorithm::Difference |
        HashAlgorithm::Perceptual => {
            // 感知哈希: 计算汉明距离的相似度
            crate::core::utils::hash_similarity(hash1, hash2)
        },
        HashAlgorithm::ORB => {
            // ORB特征匹配
            orb::calculate_orb_similarity(hash1, hash2).unwrap_or(0.0)
            // orb_hash::compare_orb_hash(hash1, hash2).unwrap_or(0.0)
        }
    }
}