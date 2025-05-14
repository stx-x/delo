use crate::core::types::HashResult;
use crate::core::utils::image_utils;
use image::GenericImageView;
use sha2::{Digest, Sha256};
use std::path::Path;

/// 计算图片的精确哈希值（SHA-256）
///
/// 精确哈希算法直接计算图像数据的SHA-256哈希值，
/// 对于完全相同的图像会生成相同的哈希值，但对图像的任何改变都非常敏感。
///
/// 这种算法适用于寻找完全相同的图像，但不适用于寻找相似的图像。
pub fn calculate_exact_hash(path: &Path) -> Result<HashResult, String> {
    // 打开图像
    let img = image_utils::open_image(path)?;
    let (width, height) = img.dimensions();

    // 使用SHA-256计算哈希值
    let mut hasher = Sha256::new();

    // 将图像数据送入哈希计算器
    hasher.update(img.into_bytes());

    // 生成最终哈希值并转换为十六进制字符串
    let hash = format!("{:x}", hasher.finalize());

    Ok(HashResult {
        hash,
        width,
        height,
    })
}

/// 比较两个精确哈希的相似度
/// 对于精确哈希，仅当哈希完全相同时返回100%，否则返回0%
pub fn compare_exact_hash(hash1: &str, hash2: &str) -> f32 {
    if hash1 == hash2 {
        100.0
    } else {
        0.0
    }
}
