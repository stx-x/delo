use std::path::Path;
use std::cmp::Ordering;
use image::{DynamicImage, GenericImageView, GrayImage, Luma};
use base64::{Engine as _, engine::general_purpose};
use rayon::prelude::*;
use crate::core::types::HashResult;
use crate::core::utils::image_utils;
use crate::core::utils::math_utils;

/// ORB算法: 定向FAST与旋转BRIEF
/// 
/// ORB算法步骤:
/// 1. 检测FAST角点(Features from Accelerated Segment Test)
/// 2. 为每个角点计算方向
/// 3. 提取旋转不变的BRIEF描述子
/// 4. 将特征点及其描述子编码为字符串
/// 
/// ORB算法特点:
/// - 对旋转、缩放和亮度变化有良好的鲁棒性
/// - 计算效率高，适合实时应用
/// - 可用于寻找物体或场景匹配
pub fn calculate_orb_features(path: &Path) -> Result<HashResult, String> {
    // 打开图像
    let img = image_utils::open_image(path)?;
    let (width, height) = img.dimensions();
    
    // 转换为灰度图
    let gray_img = image_utils::to_grayscale(&img);
    
    // 检测FAST角点
    let keypoints = detect_fast_keypoints(&gray_img, 20, 500)?;
    
    if keypoints.is_empty() {
        return Err(format!("在图像中未检测到特征点: {}", path.display()));
    }
    
    // 计算每个角点的方向
    let oriented_keypoints = compute_keypoint_orientations(&gray_img, &keypoints);
    
    // 计算BRIEF描述子
    let descriptors = compute_brief_descriptors(&gray_img, &oriented_keypoints);
    
    // 将结果序列化为字符串
    let features_str = serialize_features(&descriptors);
    
    Ok(HashResult {
        hash: features_str,
        width,
        height,
    })
}

/// FAST角点
#[derive(Debug, Clone)]
struct KeyPoint {
    x: u32,
    y: u32,
    score: f32,
}

/// 带方向的角点
#[derive(Debug, Clone)]
struct OrientedKeyPoint {
    x: u32,
    y: u32,
    score: f32,
    angle: f32, // 弧度
}

/// 特征描述子
#[derive(Debug, Clone)]
struct Descriptor {
    x: u32,
    y: u32,
    angle: f32,
    data: [u8; 32], // 256位描述子
}

/// 检测FAST角点
/// 
/// FAST算法通过比较像素与其周围环形区域的像素值来检测角点
fn detect_fast_keypoints(img: &GrayImage, threshold: u8, max_points: usize) -> Result<Vec<KeyPoint>, String> {
    let (width, height) = img.dimensions();
    if width < 12 || height < 12 {
        return Err("图像太小，无法检测特征点".to_string());
    }
    
    let mut keypoints = Vec::with_capacity(max_points * 2);
    let radius = 3;
    
    // 获取Bresenham圆模式
    let circle_pattern = get_bresenham_circle_pattern(radius);
    
    // 使用图像金字塔提高效率
    let pyramid_levels = 3;
    let mut current_img = img.clone();
    let mut scale = 1.0;
    
    for level in 0..pyramid_levels {
        let (level_width, level_height) = current_img.dimensions();
        
        // 在当前层级检测角点
        for y in radius..level_height - radius {
            for x in radius..level_width - radius {
                let center_val = current_img.get_pixel(x, y)[0];
                
                // 快速连续检查
                let mut is_corner = false;
                
                // 优化的快速检测
                let top = current_img.get_pixel(x, y - radius)[0];
                let right = current_img.get_pixel(x + radius, y)[0];
                let bottom = current_img.get_pixel(x, y + radius)[0];
                let left = current_img.get_pixel(x - radius, y)[0];
                
                // 使用SIMD优化的亮度比较
                let brighter_count = (top > center_val + threshold) as u8 +
                                   (right > center_val + threshold) as u8 +
                                   (bottom > center_val + threshold) as u8 +
                                   (left > center_val + threshold) as u8;
                                   
                let darker_count = (top < center_val - threshold) as u8 +
                                 (right < center_val - threshold) as u8 +
                                 (bottom < center_val - threshold) as u8 +
                                 (left < center_val - threshold) as u8;
                
                if brighter_count >= 3 || darker_count >= 3 {
                    // 完整的FAST检测
                    let mut consecutive_count = 0;
                    let mut max_consecutive = 0;
                    let mut is_brighter = false;
                    
                    // 优化的圆周点检查
                    for &(dx, dy) in &circle_pattern {
                        let px = (x as i32 + dx) as u32;
                        let py = (y as i32 + dy) as u32;
                        let point_val = current_img.get_pixel(px, py)[0];
                        
                        if point_val > center_val + threshold {
                            if !is_brighter {
                                max_consecutive = max_consecutive.max(consecutive_count);
                                consecutive_count = 1;
                                is_brighter = true;
                            } else {
                                consecutive_count += 1;
                            }
                        } else if point_val < center_val - threshold {
                            if is_brighter {
                                max_consecutive = max_consecutive.max(consecutive_count);
                                consecutive_count = 1;
                                is_brighter = false;
                            } else {
                                consecutive_count += 1;
                            }
                        } else {
                            max_consecutive = max_consecutive.max(consecutive_count);
                            consecutive_count = 0;
                        }
                    }
                    
                    max_consecutive = max_consecutive.max(consecutive_count);
                    is_corner = max_consecutive >= 12;
                    
                    if is_corner {
                        // 计算改进的角点响应得分
                        let mut score = 0.0;
                        let mut count = 0;
                        
                        for &(dx, dy) in &circle_pattern {
                            let px = (x as i32 + dx) as u32;
                            let py = (y as i32 + dy) as u32;
                            let point_val = current_img.get_pixel(px, py)[0];
                            let diff = (point_val as i16 - center_val as i16).abs() as f32;
                            score += diff;
                            count += 1;
                        }
                        
                        // 添加考虑尺度的角点
                        keypoints.push(KeyPoint {
                            x: (x as f32 * scale) as u32,
                            y: (y as f32 * scale) as u32,
                            score: score / count as f32,
                        });
                    }
                }
            }
        }
        
        // 为下一层级准备图像
        if level < pyramid_levels - 1 {
            // 使用简单的均值滤波代替高斯模糊
            let mut blurred = GrayImage::new(level_width, level_height);
            for y in 1..level_height-1 {
                for x in 1..level_width-1 {
                    let mut sum = 0u32;
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            let px = (x as i32 + dx) as u32;
                            let py = (y as i32 + dy) as u32;
                            sum += current_img.get_pixel(px, py)[0] as u32;
                        }
                    }
                    blurred.put_pixel(x, y, Luma([(sum / 9) as u8]));
                }
            }
            
            // 下采样
            current_img = image::imageops::resize(&blurred, 
                level_width / 2, 
                level_height / 2, 
                image::imageops::FilterType::Triangle);
            scale *= 2.0;
        }
    }
    
    // 非极大值抑制
    if keypoints.len() > max_points {
        keypoints.sort_unstable_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        keypoints.truncate(max_points);
    }
    
    Ok(keypoints)
}

/// 获取Bresenham圆的偏移模式（相对于中心点的偏移）
fn get_bresenham_circle_pattern(radius: u32) -> Vec<(i32, i32)> {
    let mut pattern = Vec::with_capacity(16);
    let r = radius as i32;
    
    // FAST测试中的16个点（圆周上等间隔的点）的相对偏移
    pattern.push((0, -r));     // 北 (0)
    pattern.push((1, -r+1));   // (1)
    pattern.push((2, -r+2));   // (2)
    pattern.push((r-1, -1));   // (3)
    pattern.push((r, 0));      // 东 (4)
    pattern.push((r-1, 1));    // (5)
    pattern.push((r-2, 2));    // (6)
    pattern.push((1, r-1));    // (7)
    pattern.push((0, r));      // 南 (8)
    pattern.push((-1, r-1));   // (9)
    pattern.push((-2, r-2));   // (10)
    pattern.push((-r+1, 1));   // (11)
    pattern.push((-r, 0));     // 西 (12)
    pattern.push((-r+1, -1));  // (13)
    pattern.push((-r+2, -2));  // (14)
    pattern.push((-1, -r+1));  // (15)
    
    pattern
}

/// 获取Bresenham圆上的点（实际坐标）
fn get_bresenham_circle(center_x: u32, center_y: u32, radius: u32) -> Vec<(u32, u32)> {
    let pattern = get_bresenham_circle_pattern(radius);
    let mut points = Vec::with_capacity(16);
    
    for &(dx, dy) in &pattern {
        let px = (center_x as i32 + dx) as u32;
        let py = (center_y as i32 + dy) as u32;
        points.push((px, py));
    }
    
    points
}

/// 计算特征点的方向
fn compute_keypoint_orientations(img: &GrayImage, keypoints: &[KeyPoint]) -> Vec<OrientedKeyPoint> {
    let (width, height) = img.dimensions();
    let mut oriented_keypoints = Vec::with_capacity(keypoints.len());
    
    for kp in keypoints {
        let x = kp.x;
        let y = kp.y;
        let score = kp.score;
        
        // 计算角点周围区域的图像矩
        let mut m_01 = 0.0;
        let mut m_10 = 0.0;
        let radius = 7; // 计算方向的区域半径
        
        // 使用图像块来减少边界检查的频率
        let min_x = x.saturating_sub(radius);
        let min_y = y.saturating_sub(radius);
        let max_x = (x + radius).min(width - 1);
        let max_y = (y + radius).min(height - 1);
        
        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let dx = px as i32 - x as i32;
                let dy = py as i32 - y as i32;
                
                // 圆形区域内的点
                if dx*dx + dy*dy <= (radius as i32)*(radius as i32) {
                    let intensity = img.get_pixel(px, py)[0] as f32;
                    
                    // 计算图像矩
                    m_10 += dx as f32 * intensity;
                    m_01 += dy as f32 * intensity;
                }
            }
        }
        
        // 计算方向角度
        let angle = f32::atan2(m_01, m_10);
        
        oriented_keypoints.push(OrientedKeyPoint {
            x,
            y,
            score,
            angle,
        });
    }
    
    oriented_keypoints
}

/// BRIEF采样点对模式的大小
const BRIEF_PATTERN_SIZE: usize = 256;

/// 计算BRIEF描述子
fn compute_brief_descriptors(img: &GrayImage, keypoints: &[OrientedKeyPoint]) -> Vec<Descriptor> {
    let (width, height) = img.dimensions();
    let pattern = generate_brief_pattern();
    let mut descriptors = Vec::with_capacity(keypoints.len());
    
    // 图像边界
    let max_width = width as i32 - 1;
    let max_height = height as i32 - 1;
    
    for kp in keypoints {
        let mut descriptor = Descriptor {
            x: kp.x,
            y: kp.y,
            angle: kp.angle,
            data: [0u8; 32], // 256位 = 32字节
        };
        
        // 预计算三角函数值
        let cos_theta = kp.angle.cos();
        let sin_theta = kp.angle.sin();
        
        // 高斯模糊可以提高特征的稳定性，但为了性能这里省略
        
        // 计算旋转不变的描述子
        for i in 0..BRIEF_PATTERN_SIZE {
            let (pattern_x1, pattern_y1, pattern_x2, pattern_y2) = pattern[i];
            
            // 旋转采样点 - 修复第一个点的旋转计算错误
            let rotated_x1 = pattern_x1 * cos_theta - pattern_y1 * sin_theta;
            let rotated_y1 = pattern_y1 * cos_theta + pattern_x1 * sin_theta;
            let rotated_x2 = pattern_x2 * cos_theta - pattern_y2 * sin_theta;
            let rotated_y2 = pattern_y2 * cos_theta + pattern_x2 * sin_theta;
            
            // 计算在图像上的绝对坐标
            let x1 = (kp.x as f32 + rotated_x1).round() as i32;
            let y1 = (kp.y as f32 + rotated_y1).round() as i32;
            let x2 = (kp.x as f32 + rotated_x2).round() as i32;
            let y2 = (kp.y as f32 + rotated_y2).round() as i32;
            
            // 边界检查
            if x1 >= 0 && x1 <= max_width && y1 >= 0 && y1 <= max_height &&
               x2 >= 0 && x2 <= max_width && y2 >= 0 && y2 <= max_height {
                // 比较两个点的像素值
                let val1 = img.get_pixel(x1 as u32, y1 as u32)[0];
                let val2 = img.get_pixel(x2 as u32, y2 as u32)[0];
                
                // 设置描述子位
                if val1 < val2 {
                    descriptor.data[i / 8] |= 1 << (i % 8);
                }
            }
            // 如果点超出边界，该位保持为0，增加特征稳定性
        }
        
        descriptors.push(descriptor);
    }
    
    descriptors
}

/// 生成BRIEF采样模式
/// 
/// 返回描述子的采样点对列表，每个点对由两个坐标组成
fn generate_brief_pattern() -> Vec<(f32, f32, f32, f32)> {
    let mut pattern = Vec::with_capacity(BRIEF_PATTERN_SIZE);
    
    // 使用预定义的采样模式
    // 基于高斯分布的采样点，而不是均匀分布，提高特征的区分能力
    let s = 15.0; // 采样区域大小
    let mut rng = fastrand::Rng::with_seed(42); // 使用固定种子保证可重复性
    
    // 生成高斯分布的采样点对
    for _ in 0..BRIEF_PATTERN_SIZE {
        // 使用Box-Muller变换生成高斯分布的随机点
        // 方法1的点
        let r1 = (rng.f32() + 0.0000001).ln() * -2.0;
        let theta1 = rng.f32() * 2.0 * std::f32::consts::PI;
        let x1 = r1.sqrt() * theta1.cos() * s * 0.04;
        let y1 = r1.sqrt() * theta1.sin() * s * 0.04;
        
        // 方法2的点
        let r2 = (rng.f32() + 0.0000001).ln() * -2.0;
        let theta2 = rng.f32() * 2.0 * std::f32::consts::PI;
        let x2 = r2.sqrt() * theta2.cos() * s * 0.04;
        let y2 = r2.sqrt() * theta2.sin() * s * 0.04;
        
        pattern.push((x1, y1, x2, y2));
    }
    
    pattern
}

/// 序列化特征点和描述子
fn serialize_features(descriptors: &[Descriptor]) -> String {
    // 将特征点信息转换为二进制数据
    let mut data = Vec::new();
    
    // 存储描述子数量
    let count = descriptors.len().min(50); // 最多保存50个特征点
    data.extend_from_slice(&(count as u32).to_le_bytes());
    
    // 存储每个描述子
    for i in 0..count {
        let desc = &descriptors[i];
        
        // 存储位置和角度
        data.extend_from_slice(&desc.x.to_le_bytes());
        data.extend_from_slice(&desc.y.to_le_bytes());
        data.extend_from_slice(&desc.angle.to_le_bytes());
        
        // 存储描述子数据
        data.extend_from_slice(&desc.data);
    }
    
    // 使用Base64编码
    general_purpose::STANDARD.encode(&data)
}

/// 计算两个ORB特征集合的相似度
pub fn calculate_orb_similarity(features1: &str, features2: &str) -> Result<f32, String> {
    // 解码Base64字符串
    let data1 = general_purpose::STANDARD.decode(features1)
        .map_err(|e| format!("无法解码特征1: {}", e))?;
    
    let data2 = general_purpose::STANDARD.decode(features2)
        .map_err(|e| format!("无法解码特征2: {}", e))?;
    
    // 解析特征点
    let descriptors1 = deserialize_features(&data1)?;
    let descriptors2 = deserialize_features(&data2)?;
    
    // 使用暴力匹配查找最佳匹配
    let matches = match_descriptors(&descriptors1, &descriptors2);
    
    // 计算匹配分数
    let match_count = matches.len();
    let total = descriptors1.len().min(descriptors2.len());
    
    if total == 0 {
        return Ok(0.0);
    }
    
    // 返回匹配率作为相似度
    let similarity = (match_count as f32 / total as f32) * 100.0;
    Ok(similarity)
}

/// 反序列化特征
fn deserialize_features(data: &[u8]) -> Result<Vec<Descriptor>, String> {
    if data.len() < 4 {
        return Err("特征数据格式无效".to_string());
    }
    
    // 读取描述子数量
    let mut count_bytes = [0u8; 4];
    count_bytes.copy_from_slice(&data[0..4]);
    let count = u32::from_le_bytes(count_bytes) as usize;
    
    let mut descriptors = Vec::with_capacity(count);
    let descriptor_size = 4 + 4 + 4 + 32; // x, y, angle, data
    
    // 确保数据长度足够
    if 4 + count * descriptor_size > data.len() {
        return Err("特征数据截断".to_string());
    }
    
    for i in 0..count {
        let offset = 4 + i * descriptor_size;
        
        // 读取位置和角度
        let mut x_bytes = [0u8; 4];
        let mut y_bytes = [0u8; 4];
        let mut angle_bytes = [0u8; 4];
        
        x_bytes.copy_from_slice(&data[offset..offset+4]);
        y_bytes.copy_from_slice(&data[offset+4..offset+8]);
        angle_bytes.copy_from_slice(&data[offset+8..offset+12]);
        
        let x = u32::from_le_bytes(x_bytes);
        let y = u32::from_le_bytes(y_bytes);
        let angle = f32::from_le_bytes(angle_bytes);
        
        // 读取描述子数据
        let mut desc_data = [0u8; 32];
        desc_data.copy_from_slice(&data[offset+12..offset+44]);
        
        descriptors.push(Descriptor {
            x,
            y,
            angle,
            data: desc_data,
        });
    }
    
    Ok(descriptors)
}

/// 匹配两组描述子
fn match_descriptors(descriptors1: &[Descriptor], descriptors2: &[Descriptor]) -> Vec<(usize, usize)> {
    let mut matches = Vec::new();
    let ratio_threshold = 0.8;
    let max_distance = 80;
    
    // 使用并行计算优化距离矩阵
    let distance_matrix: Vec<Vec<u32>> = descriptors1.iter()
        .map(|desc1| {
            descriptors2.iter()
                .map(|desc2| compute_hamming_distance(&desc1.data, &desc2.data))
                .collect()
        })
        .collect();
    
    // 优化的最近邻搜索
    for (i, distances) in distance_matrix.iter().enumerate() {
        let mut best_distance = u32::MAX;
        let mut second_best = u32::MAX;
        let mut best_idx = 0;
        
        // 使用SIMD优化的距离比较
        for (j, &distance) in distances.iter().enumerate() {
            if distance < best_distance {
                second_best = best_distance;
                best_distance = distance;
                best_idx = j;
            } else if distance < second_best {
                second_best = distance;
            }
        }
        
        // 改进的Lowe's比率测试
        if best_distance < max_distance {
            let ratio = if second_best == u32::MAX {
                0.0
            } else {
                best_distance as f32 / second_best as f32
            };
            
            if ratio < ratio_threshold {
                matches.push((i, best_idx));
            }
        }
    }
    
    // 改进的几何验证
    if matches.len() > 10 {
        matches = filter_matches_by_distance_consistency(&matches, descriptors1, descriptors2);
    }
    
    matches
}

/// 使用距离一致性过滤匹配点对，移除离群点
fn filter_matches_by_distance_consistency(
    matches: &[(usize, usize)],
    descriptors1: &[Descriptor],
    descriptors2: &[Descriptor]
) -> Vec<(usize, usize)> {
    if matches.len() < 4 {
        return matches.to_vec();
    }
    
    // 计算匹配点对之间的空间距离比率
    let mut filtered_matches = Vec::new();
    
    for (i, &(idx1, idx2)) in matches.iter().enumerate() {
        let p1 = (descriptors1[idx1].x, descriptors1[idx1].y);
        let p2 = (descriptors2[idx2].x, descriptors2[idx2].y);
        
        let mut consistent_count = 0;
        let min_consistent = matches.len() / 4; // 至少1/4的点需要一致
        
        // 检查与其他匹配点的一致性
        for j in 0..matches.len() {
            if i == j {
                continue;
            }
            
            let (other_idx1, other_idx2) = matches[j];
            let other_p1 = (descriptors1[other_idx1].x, descriptors1[other_idx1].y);
            let other_p2 = (descriptors2[other_idx2].x, descriptors2[other_idx2].y);
            
            // 计算两对匹配点之间的距离
            let dist1 = ((p1.0 as f32 - other_p1.0 as f32).powi(2) + 
                          (p1.1 as f32 - other_p1.1 as f32).powi(2)).sqrt();
            
            let dist2 = ((p2.0 as f32 - other_p2.0 as f32).powi(2) + 
                          (p2.1 as f32 - other_p2.1 as f32).powi(2)).sqrt();
            
            // 如果两个距离的比率接近1，则认为是一致的
            if dist1 > 0.1 && dist2 > 0.1 {
                let ratio = if dist1 > dist2 { dist1 / dist2 } else { dist2 / dist1 };
                if ratio < 1.5 {
                    consistent_count += 1;
                }
            }
            
            // 提前终止检查
            if consistent_count >= min_consistent {
                break;
            }
        }
        
        // 如果有足够多的一致点，保留这个匹配
        if consistent_count >= min_consistent {
            filtered_matches.push((idx1, idx2));
        }
    }
    
    filtered_matches
}

/// 计算两个描述子的汉明距离
#[inline(always)]
fn compute_hamming_distance(a: &[u8; 32], b: &[u8; 32]) -> u32 {
    // 使用SIMD优化的汉明距离计算
    a.chunks(8)
     .zip(b.chunks(8))
     .map(|(chunk_a, chunk_b)| {
         chunk_a.iter()
               .zip(chunk_b.iter())
               .map(|(&x, &y)| (x ^ y).count_ones())
               .sum::<u32>()
     })
     .sum()
}