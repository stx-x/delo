pub mod file_utils;
pub mod image_utils;
pub mod math_utils;
pub mod hash_utils;

// 重新导出常用工具函数
pub use file_utils::*;
pub use image_utils::*;
pub use math_utils::*;
pub use hash_utils::*;
 
/// 计算两个二进制哈希字符串之间的汉明距离
pub fn hamming_distance(hash1: &str, hash2: &str) -> u32 {
    hash1.chars()
        .zip(hash2.chars())
        .filter(|(a, b)| a != b)
        .count() as u32
}

/// 计算两个二进制数组之间的汉明距离
pub fn hamming_distance_bytes(bytes1: &[u8], bytes2: &[u8]) -> u32 {
    bytes1.iter()
        .zip(bytes2.iter())
        .map(|(a, b)| (a ^ b).count_ones())
        .sum()
}

/// 计算两个哈希值的相似度百分比 (0-100)
pub fn hash_similarity(hash1: &str, hash2: &str) -> f32 {
    let distance = hamming_distance(hash1, hash2);
    let max_distance = hash1.len() as f32;
    100.0 * (1.0 - (distance as f32 / max_distance))
}