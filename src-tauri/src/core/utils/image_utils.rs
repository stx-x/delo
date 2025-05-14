use std::path::Path;
use image::{DynamicImage, GenericImageView, imageops::FilterType, GrayImage};

/// 打开图像文件
pub fn open_image(path: &Path) -> Result<DynamicImage, String> {
    image::open(path)
        .map_err(|e| format!("无法打开图片 {}: {}", path.display(), e))
}

/// 将图像调整为指定大小
pub fn resize_image(img: &DynamicImage, width: u32, height: u32) -> DynamicImage {
    img.resize_exact(width, height, FilterType::Lanczos3)
}

/// 将图像转换为灰度图
pub fn to_grayscale(img: &DynamicImage) -> GrayImage {
    img.to_luma8()
}

/// 计算灰度图像的平均像素值
pub fn average_pixel_value(img: &GrayImage) -> u8 {
    let sum: u32 = img.pixels().map(|p| p[0] as u32).sum();
    (sum / (img.width() * img.height())) as u8
}

/// 从灰度图像生成比特串
pub fn generate_bits_from_threshold(img: &GrayImage, threshold: u8) -> String {
    img.pixels()
        .map(|p| if p[0] > threshold { '1' } else { '0' })
        .collect()
}

/// 将比特串转换为字节数组
pub fn bits_to_bytes(bits: &str) -> Vec<u8> {
    bits.as_bytes()
        .chunks(8)
        .map(|chunk| {
            chunk.iter()
                .enumerate()
                .fold(0u8, |acc, (i, &b)| {
                    if b == b'1' {
                        acc | (1 << (7 - i))
                    } else {
                        acc
                    }
                })
        })
        .collect()
}

/// 图像矩阵类型 - 表示灰度图像的浮点数值
pub type ImageMatrix = Vec<Vec<f64>>;

/// 将灰度图像转换为浮点数矩阵
pub fn gray_image_to_matrix(img: &GrayImage) -> ImageMatrix {
    let (width, height) = img.dimensions();
    let mut matrix = vec![vec![0.0f64; width as usize]; height as usize];
    
    for y in 0..height {
        for x in 0..width {
            matrix[y as usize][x as usize] = img.get_pixel(x, y)[0] as f64;
        }
    }
    
    matrix
}

/// 将浮点数矩阵转换为灰度图像
pub fn matrix_to_gray_image(matrix: &ImageMatrix) -> GrayImage {
    let height = matrix.len();
    let width = matrix[0].len();
    
    let mut img = GrayImage::new(width as u32, height as u32);
    
    for y in 0..height {
        for x in 0..width {
            let value = matrix[y][x].round().clamp(0.0, 255.0) as u8;
            img.put_pixel(x as u32, y as u32, image::Luma([value]));
        }
    }
    
    img
}

/// 对图像矩阵应用阈值
pub fn threshold_matrix(matrix: &ImageMatrix, threshold: f64) -> String {
    let mut bits = String::with_capacity(matrix.len() * matrix[0].len());
    
    for row in matrix {
        for &value in row {
            bits.push(if value > threshold { '1' } else { '0' });
        }
    }
    
    bits
}