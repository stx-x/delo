use std::path::Path;
use sha2::{Sha256, Digest};
use crate::core::types::HashAlgorithm;
use base64::{Engine as _, engine::general_purpose};

/// 将二进制数据序列化为Base64字符串
pub fn serialize_to_base64(data: &[u8]) -> String {
    general_purpose::STANDARD.encode(data)
}

/// 从Base64字符串反序列化为二进制数据
pub fn deserialize_from_base64(encoded: &str) -> Result<Vec<u8>, String> {
    general_purpose::STANDARD.decode(encoded)
        .map_err(|e| format!("Base64解码失败: {}", e))
}

/// 计算文件的SHA-256哈希值
pub fn compute_file_sha256(path: &Path) -> Result<String, String> {
    let data = std::fs::read(path)
        .map_err(|e| format!("读取文件失败: {}", e))?;
    
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let result = hasher.finalize();
    
    Ok(format!("{:x}", result))
}

/// 计算二进制数据的SHA-256哈希值
pub fn compute_data_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// 计算两个哈希字符串的相似度(0-100)
pub fn calculate_similarity(hash1: &str, hash2: &str, algorithm: HashAlgorithm) -> f32 {
    match algorithm {
        HashAlgorithm::Exact => {
            // 精确哈希: 完全相同返回100%, 否则返回0%
            if hash1 == hash2 { 100.0 } else { 0.0 }
        },
        HashAlgorithm::ORB => {
            // ORB算法: 需要解码特征点并计算匹配
            match compute_orb_similarity(hash1, hash2) {
                Ok(similarity) => similarity,
                Err(_) => 0.0,
            }
        },
        _ => {
            // 感知哈希算法: 计算汉明距离的相似度
            let distance = super::hamming_distance(hash1, hash2);
            let max_distance = hash1.len() as f32;
            100.0 * (1.0 - (distance as f32 / max_distance))
        }
    }
}

/// 计算ORB特征匹配的相似度
fn compute_orb_similarity(features1: &str, features2: &str) -> Result<f32, String> {
    // 调用ORB算法模块中的相似度计算函数
    crate::algorithms::orb::calculate_orb_similarity(features1, features2)
    // crate::algorithms::orb::compare_orb_hash(features1, features2)
}

/// 将哈希字符串分割成多个片段(用于LSH算法)
pub fn split_hash_for_lsh(hash: &str, num_bands: usize) -> Vec<String> {
    let band_size = hash.len() / num_bands;
    
    // 如果哈希值长度不是num_bands的整数倍，就舍去末尾的一些字符
    (0..num_bands)
        .map(|i| {
            let start = i * band_size;
            let end = (i + 1) * band_size;
            
            if end <= hash.len() {
                hash[start..end].to_string()
            } else if start < hash.len() {
                hash[start..].to_string()
            } else {
                String::new()
            }
        })
        .filter(|s| !s.is_empty())
        .collect()
}