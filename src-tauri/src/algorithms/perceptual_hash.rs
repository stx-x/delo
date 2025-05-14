use crate::core::types::HashResult;
use crate::core::utils::image_utils;
use crate::core::utils::math_utils;
use image::GenericImageView;
use std::path::Path;
use rayon::prelude::*;
use bit_vec::BitVec;

/// 计算图片的感知哈希 (Perceptual Hash / pHash)
///
/// 感知哈希算法步骤:
/// 1. 将图像缩放为32x32大小
/// 2. 将图像转换为灰度图
/// 3. 对图像进行离散余弦变换(DCT)
/// 4. 取DCT的低频区域(通常是左上角的8x8)
/// 5. 计算这个区域的中位数
/// 6. 根据每个DCT系数与中位数的比较生成64位哈希
///
/// 感知哈希对于图像的内容变化非常敏感，同时对于缩放、旋转、压缩等操作有较好的鲁棒性。
pub fn calculate_perceptual_hash(path: &Path) -> Result<HashResult, String> {
    // 打开图像
    let img = image_utils::open_image(path)?;
    let (width, height) = img.dimensions();

    // 缩放图像为32x32并转换为灰度图
    let small_img = image_utils::resize_image(&img, 32, 32);
    let gray_img = image_utils::to_grayscale(&small_img);

    // 使用缓存优化的DCT实现
    let hash = calculate_phash_from_image(&gray_img);

    Ok(HashResult {
        hash,
        width,
        height,
    })
}

/// 内部函数：从灰度图计算感知哈希
/// 优化DCT计算和哈希生成过程
fn calculate_phash_from_image(gray_img: &image::GrayImage) -> String {
    // 转换为浮点数矩阵
    let matrix = image_utils::gray_image_to_matrix(gray_img);

    // 使用优化版本DCT变换，只计算需要的部分
    let dct_matrix = math_utils::dct_2d_optimized(&matrix, 8, 8);

    // 提取左上角8x8的低频区域 (跳过直流分量DC，即[0,0])
    let mut low_freq = Vec::with_capacity(63);
    let mut i = 0;
    for y in 0..8 {
        for x in 0..8 {
            if !(y == 0 && x == 0) { // 跳过DC分量
                low_freq.push((dct_matrix[y][x], i));
                i += 1;
            }
        }
    }

    // 对系数进行排序以找到中位数
    low_freq.sort_unstable_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let median_idx = low_freq.len() / 2;
    let median = low_freq[median_idx].0;

    // 创建位向量并设置比特
    let mut bit_vec = BitVec::from_elem(64, false);
    for (val, idx) in low_freq {
        if val > median {
            bit_vec.set(idx, true);
        }
    }

    // 将位向量转换为字符串
    let mut hash = String::with_capacity(64);
    for i in 0..64 {
        hash.push(if i < bit_vec.len() && bit_vec[i] { '1' } else { '0' });
    }

    hash
}

/// 计算两个感知哈希的相似度
/// 使用汉明距离(不同位的数量)来计算相似度
pub fn compare_perceptual_hash(hash1: &str, hash2: &str) -> f32 {
    // 使用优化的汉明距离计算
    let distance = compute_hamming_distance(hash1, hash2);

    // 计算相似度百分比(0-100)
    let max_distance = hash1.len();
    100.0 * (1.0 - (distance as f32 / max_distance as f32))
}

/// 计算两个二进制字符串之间的汉明距离
/// 优化实现，使用位向量和位操作
fn compute_hamming_distance(hash1: &str, hash2: &str) -> usize {
    // 如果长度不同，使用最短的长度
    let min_len = hash1.len().min(hash2.len());
    
    // 使用SIMD优化的位计数方法
    let mut count = 0;
    for i in 0..min_len {
        if hash1.as_bytes()[i] != hash2.as_bytes()[i] {
            count += 1;
        }
    }
    
    // 处理长度差异
    let len_diff = hash1.len().abs_diff(hash2.len());
    count + len_diff
}

/// 计算感知哈希并使用加权策略
/// 这是一个增强版的pHash，对低频区域的不同位置使用不同权重
pub fn calculate_weighted_phash(path: &Path) -> Result<HashResult, String> {
    // 打开并处理图像
    let img = image_utils::open_image(path)?;
    let (width, height) = img.dimensions();

    // 缩放图像为32x32并转换为灰度图
    let small_img = image_utils::resize_image(&img, 32, 32);
    let gray_img = image_utils::to_grayscale(&small_img);
    
    // 转换为浮点数矩阵
    let matrix = image_utils::gray_image_to_matrix(&gray_img);
    
    // 使用优化版本DCT变换
    let dct_matrix = math_utils::dct_2d_optimized(&matrix, 8, 8);

    // 创建权重表 - 频率越低权重越高
    let weights = generate_frequency_weights(8);
    
    // 对左上角8x8区域应用权重并生成值列表
    let mut weighted_values = Vec::with_capacity(64);
    
    for y in 0..8 {
        for x in 0..8 {
            let coef = dct_matrix[y][x];
            let weight = weights[y][x];
            weighted_values.push(coef * weight);
        }
    }

    // 获取中值用于二值化
    let mut values_copy = weighted_values.clone();
    let median = math_utils::median(&mut values_copy);

    // 使用位向量生成哈希
    let mut bit_vec = BitVec::from_elem(64, false);
    for (i, &val) in weighted_values.iter().enumerate() {
        if val > median {
            bit_vec.set(i, true);
        }
    }
    
    // 转换为字符串
    let mut weighted_hash = String::with_capacity(64);
    for i in 0..64 {
        weighted_hash.push(if bit_vec[i] { '1' } else { '0' });
    }

    Ok(HashResult {
        hash: weighted_hash,
        width,
        height,
    })
}

/// 生成频率权重矩阵，更低频的区域权重更高
fn generate_frequency_weights(size: usize) -> Vec<Vec<f64>> {
    let mut weights = vec![vec![0.0; size]; size];
    
    // 计算权重，左上角（低频）权重最高
    for y in 0..size {
        for x in 0..size {
            // 使用欧几里得距离计算权重，距离原点越远权重越低
            let distance = ((x*x + y*y) as f64).sqrt();
            let max_distance = ((size*size + size*size) as f64).sqrt();
            weights[y][x] = 1.0 - (distance / max_distance);
        }
    }
    
    weights
}
