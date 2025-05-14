use std::path::Path;
use image::GenericImageView;
use crate::core::types::HashResult;
use crate::core::utils::image_utils;

/// 计算图片的差值哈希 (Difference Hash / dHash)
/// 
/// 差值哈希算法步骤:
/// 1. 将图像缩放为9x8大小(比均值哈希多一列用于计算相邻像素差异)
/// 2. 将图像转换为灰度图
/// 3. 计算相邻像素的差值
/// 4. 根据差值的正负生成64位哈希
/// 
/// 相比均值哈希，差值哈希能更好地捕捉图像的纹理特征和边缘信息。
pub fn calculate_difference_hash(path: &Path) -> Result<HashResult, String> {
    // 打开图像
    let img = image_utils::open_image(path)?;
    let (width, height) = img.dimensions();
    
    // 缩放图像为9x8 (多一列用于比较差值)
    let small_img = image_utils::resize_image(&img, 9, 8);
    
    // 转换为灰度图
    let gray_img = image_utils::to_grayscale(&small_img);
    
    // 生成哈希值
    let mut hash = String::with_capacity(64);
    
    // 比较相邻像素生成差值哈希
    for y in 0..8 {
        for x in 0..8 {
            let current = gray_img.get_pixel(x, y)[0];
            let next = gray_img.get_pixel(x + 1, y)[0];
            
            // 如果当前像素比下一个像素亮，则为1，否则为0
            hash.push(if current > next { '1' } else { '0' });
        }
    }
    
    Ok(HashResult {
        hash,
        width,
        height,
    })
}

/// 计算两个差值哈希的相似度
/// 使用汉明距离(不同位的数量)来计算相似度
pub fn compare_difference_hash(hash1: &str, hash2: &str) -> f32 {
    // 计算汉明距离
    let distance = hash1.chars()
        .zip(hash2.chars())
        .filter(|(a, b)| a != b)
        .count();
    
    // 计算相似度百分比(0-100)
    let max_distance = hash1.len();
    100.0 * (1.0 - (distance as f32 / max_distance as f32))
}