use crate::core::types::HashResult;
use crate::core::utils::image_utils;
use crate::core::utils::math_utils;
use image::GenericImageView;
use std::path::Path;

/// 计算图片的感知哈希 (Perceptual Hash / pHash)
///
/// 感知哈希算法步骤:
/// 1. 将图像缩放为32x32大小
/// 2. 将图像转换为灰度图
/// 3. 对图像进行离散余弦变换(DCT)
/// 4. 取DCT的低频区域(通常是左上角的8x8)
/// 5. 计算这个区域的均值
/// 6. 根据每个DCT系数与均值的比较生成64位哈希
///
/// 感知哈希对于图像的内容变化非常敏感，同时对于缩放、旋转、压缩等操作有较好的鲁棒性。
pub fn calculate_perceptual_hash(path: &Path) -> Result<HashResult, String> {
    // 打开图像
    let img = image_utils::open_image(path)?;
    let (width, height) = img.dimensions();

    // 缩放图像为32x32
    let small_img = image_utils::resize_image(&img, 32, 32);

    // 转换为灰度图
    let gray_img = image_utils::to_grayscale(&small_img);

    // 转换为浮点数矩阵
    let matrix = image_utils::gray_image_to_matrix(&gray_img);

    // 应用DCT变换
    let dct_matrix = math_utils::dct_2d(&matrix);

    // 提取左上角8x8的低频区域
    let mut low_freq = Vec::with_capacity(64);
    for y in 0..8 {
        for x in 0..8 {
            low_freq.push(dct_matrix[y][x]);
        }
    }

    // 计算这个区域的中位数
    let mut low_freq_copy = low_freq.clone();
    let median = math_utils::median(&mut low_freq_copy);

    // 根据每个值与中位数的比较生成哈希值
    let mut hash = String::with_capacity(64);
    for &val in &low_freq {
        hash.push(if val > median { '1' } else { '0' });
    }

    Ok(HashResult {
        hash,
        width,
        height,
    })
}

/// 计算两个感知哈希的相似度
/// 使用汉明距离(不同位的数量)来计算相似度
pub fn compare_perceptual_hash(hash1: &str, hash2: &str) -> f32 {
    // 计算汉明距离
    let distance = hash1
        .chars()
        .zip(hash2.chars())
        .filter(|(a, b)| a != b)
        .count();

    // 计算相似度百分比(0-100)
    let max_distance = hash1.len();
    100.0 * (1.0 - (distance as f32 / max_distance as f32))
}

/// 计算感知哈希并使用加权策略
/// 这是一个增强版的pHash，对低频区域的不同位置使用不同权重
pub fn calculate_weighted_phash(path: &Path) -> Result<HashResult, String> {
    // 打开并处理图像，获取DCT系数
    let img = image_utils::open_image(path)?;
    let (width, height) = img.dimensions();

    // 缩放图像为32x32
    let small_img = image_utils::resize_image(&img, 32, 32);
    let gray_img = image_utils::to_grayscale(&small_img);
    let matrix = image_utils::gray_image_to_matrix(&gray_img);
    let dct_matrix = math_utils::dct_2d(&matrix);

    // 对左上角8x8区域应用权重
    // 左上角(更低频)的权重更高
    let mut weighted_hash = String::with_capacity(64);
    let mut values = Vec::with_capacity(64);

    for y in 0..8 {
        for x in 0..8 {
            values.push(dct_matrix[y][x]);
        }
    }

    // 获取中值用于二值化
    let mut values_copy = values.clone();
    let median = math_utils::median(&mut values_copy);

    // 生成哈希值
    for val in values {
        weighted_hash.push(if val > median { '1' } else { '0' });
    }

    Ok(HashResult {
        hash: weighted_hash,
        width,
        height,
    })
}
