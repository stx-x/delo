use std::path::Path;
use image::GenericImageView;
use crate::core::types::HashResult;
use crate::core::utils::image_utils;

/// 计算图片的均值哈希 (Average Hash / aHash)
/// 
/// 均值哈希算法步骤:
/// 1. 将图像缩放为8x8大小(去除高频细节与图像大小依赖)
/// 2. 将图像转换为灰度图
/// 3. 计算灰度图像的平均值
/// 4. 根据每个像素与平均值的比较生成64位哈希
/// 
/// 这种算法对于缩放和小变化具有一定的鲁棒性。
pub fn calculate_average_hash(path: &Path) -> Result<HashResult, String> {
    // 打开图像
    let img = image_utils::open_image(path)?;
    let (width, height) = img.dimensions();
    
    // 缩放图像为8x8
    let small_img = image_utils::resize_image(&img, 8, 8);
    
    // 转换为灰度图
    let gray_img = image_utils::to_grayscale(&small_img);
    
    // 计算平均像素值
    let average = image_utils::average_pixel_value(&gray_img);
    
    // 生成哈希值
    let hash = image_utils::generate_bits_from_threshold(&gray_img, average);
    
    Ok(HashResult {
        hash,
        width, 
        height,
    })
}

/// 计算两个均值哈希的相似度
/// 使用汉明距离(不同位的数量)来计算相似度
pub fn compare_average_hash(hash1: &str, hash2: &str) -> f32 {
    // 计算汉明距离
    let distance = hash1.chars()
        .zip(hash2.chars())
        .filter(|(a, b)| a != b)
        .count();
    
    // 计算相似度百分比(0-100)
    let max_distance = hash1.len();
    100.0 * (1.0 - (distance as f32 / max_distance as f32))
}