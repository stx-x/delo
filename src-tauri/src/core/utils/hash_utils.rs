use std::path::Path;
use sha2::{Sha256, Digest};
use crate::core::types::HashAlgorithm;
use base64::{Engine as _, engine::general_purpose};
use bit_vec::BitVec;

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
}

/// 将哈希字符串分割成多个片段(用于LSH算法)
pub fn split_hash_for_lsh(hash: &str, num_bands: usize) -> Vec<String> {
    let band_size = hash.len() / num_bands;
    if band_size == 0 {
        return vec![hash.to_string()];
    }
    
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

/// 将哈希字符串转换为二进制位向量，用于高效处理
pub fn hash_string_to_bitvec(hash: &str) -> BitVec {
    // 根据哈希类型选择不同的转换策略
    if hash.len() >= 64 && hash.chars().all(|c| c == '0' || c == '1') {
        // 纯二进制字符串（如感知哈希结果）
        let mut bitvec = BitVec::with_capacity(hash.len());
        for c in hash.chars() {
            bitvec.push(c == '1');
        }
        bitvec
    } else if hash.len() >= 10 && hash.starts_with("eJ") {
        // 可能是压缩的ORB特征
        convert_base64_to_bitvec(hash)
    } else if hash.len() >= 10 && is_base64(hash) {
        // 可能是标准Base64编码
        convert_base64_to_bitvec(hash)
    } else {
        // 默认处理：直接将字符转为二进制
        let mut bitvec = BitVec::with_capacity(hash.len() * 8);
        for b in hash.bytes() {
            for i in 0..8 {
                bitvec.push(((b >> (7 - i)) & 1) == 1);
            }
        }
        bitvec
    }
}

/// 检查字符串是否可能是Base64编码
fn is_base64(s: &str) -> bool {
    // 简单检查字符是否在Base64字符集中，以及长度是否合理
    s.chars().all(|c| {
        c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='
    }) && s.len() % 4 <= 1 // Base64编码长度通常是4的倍数，可能有1-3个=补齐
}

/// 将Base64编码的字符串转换为二进制位向量
fn convert_base64_to_bitvec(base64_str: &str) -> BitVec {
    // 尝试解码Base64
    if let Ok(bytes) = general_purpose::STANDARD.decode(base64_str) {
        // 将解码后的字节转为位向量
        let mut bitvec = BitVec::with_capacity(bytes.len() * 8);
        for b in bytes {
            for i in 0..8 {
                bitvec.push(((b >> (7 - i)) & 1) == 1);
            }
        }
        bitvec
    } else {
        // 解码失败，降级为普通字符串处理
        let mut bitvec = BitVec::with_capacity(base64_str.len() * 8);
        for b in base64_str.bytes() {
            for i in 0..8 {
                bitvec.push(((b >> (7 - i)) & 1) == 1);
            }
        }
        bitvec
    }
}

/// 紧凑哈希表示：将二进制哈希字符串转换为u64
pub fn binary_hash_to_u64(hash: &str) -> u64 {
    if hash.len() < 64 {
        // 如果长度小于64位，直接使用简单的哈希函数
        let mut h: u64 = 0;
        for b in hash.bytes() {
            h = h.wrapping_mul(31).wrapping_add(b as u64);
        }
        return h;
    }
    
    // 对于二进制哈希，取前64位
    let mut result: u64 = 0;
    for (i, c) in hash.chars().take(64).enumerate() {
        if c == '1' {
            result |= 1 << i;
        }
    }
    result
}