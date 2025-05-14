use std::path::Path;
use std::cmp::Ordering;
use image::{DynamicImage, GenericImageView, GrayImage};
use base64::{Engine as _, engine::general_purpose};
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
    
    let mut keypoints = Vec::new();
    let radius = 3; // FAST使用3像素半径的圆
    
    // 在图像上遍历可能的角点
    for y in radius..height - radius {
        for x in radius..width - radius {
            let center_val = img.get_pixel(x, y)[0];
            
            // 检查Bresenham圆上的16个点
            let circle_points = get_bresenham_circle(x, y, radius);
            
            // 计算连续的更亮或更暗的点的数量
            let mut brighter_count = 0;
            let mut darker_count = 0;
            let mut max_consecutive = 0;
            
            for &(px, py) in &circle_points {
                if px >= width || py >= height {
                    continue;
                }
                
                let point_val = img.get_pixel(px, py)[0];
                
                if point_val > center_val + threshold {
                    brighter_count += 1;
                    darker_count = 0;
                } else if point_val < center_val - threshold {
                    darker_count += 1;
                    brighter_count = 0;
                } else {
                    brighter_count = 0;
                    darker_count = 0;
                }
                
                max_consecutive = max_consecutive.max(brighter_count).max(darker_count);
                
                // 如果找到9个连续的更亮或更暗的点，认为是角点
                if max_consecutive >= 9 {
                    break;
                }
            }
            
            if max_consecutive >= 9 {
                // 计算角点得分
                let mut score = 0.0;
                for &(px, py) in &circle_points {
                    if px >= width || py >= height {
                        continue;
                    }
                    
                    let point_val = img.get_pixel(px, py)[0];
                    let diff = (point_val as i16 - center_val as i16).abs() as f32;
                    score += diff;
                }
                
                keypoints.push(KeyPoint {
                    x,
                    y,
                    score: score / 16.0, // 平均差异作为分数
                });
            }
        }
    }
    
    // 如果找到的角点太多，保留得分最高的一些
    if keypoints.len() > max_points {
        keypoints.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        keypoints.truncate(max_points);
    }
    
    Ok(keypoints)
}

/// 获取Bresenham圆上的点
fn get_bresenham_circle(center_x: u32, center_y: u32, radius: u32) -> Vec<(u32, u32)> {
    let mut points = Vec::with_capacity(16);
    
    // FAST测试中的16个点（圆周上等间隔的点）
    points.push((center_x, center_y - radius)); // 北
    points.push((center_x + 1, center_y - radius + 1));
    points.push((center_x + 2, center_y - radius + 2));
    points.push((center_x + radius - 1, center_y - 1));
    points.push((center_x + radius, center_y)); // 东
    points.push((center_x + radius - 1, center_y + 1));
    points.push((center_x + radius - 2, center_y + 2));
    points.push((center_x + 1, center_y + radius - 1));
    points.push((center_x, center_y + radius)); // 南
    points.push((center_x - 1, center_y + radius - 1));
    points.push((center_x - 2, center_y + radius - 2));
    points.push((center_x - radius + 1, center_y + 1));
    points.push((center_x - radius, center_y)); // 西
    points.push((center_x - radius + 1, center_y - 1));
    points.push((center_x - radius + 2, center_y - 2));
    points.push((center_x - 1, center_y - radius + 1));
    
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
        
        // 计算角点周围区域的矩
        let mut m_01 = 0.0;
        let mut m_10 = 0.0;
        let radius = 7; // 计算方向的区域半径
        
        for dy in -(radius as i32)..=(radius as i32) {
            for dx in -(radius as i32)..=(radius as i32) {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                
                // 检查边界
                if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                    let intensity = img.get_pixel(nx as u32, ny as u32)[0] as f32;
                    
                    // 计算矩
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
    
    for kp in keypoints {
        let mut descriptor = Descriptor {
            x: kp.x,
            y: kp.y,
            angle: kp.angle,
            data: [0u8; 32], // 256位 = 32字节
        };
        
        let cos_theta = kp.angle.cos();
        let sin_theta = kp.angle.sin();
        
        // 计算旋转不变的描述子
        for i in 0..BRIEF_PATTERN_SIZE {
            let (pattern_x1, pattern_y1, pattern_x2, pattern_y2) = pattern[i];
            
            // 旋转采样点
            let rotated_x1 = pattern_x1 * cos_theta - pattern_y1 * sin_theta;
            let rotated_y1 = pattern_x1 * sin_theta + pattern_y1 * cos_theta;
            let rotated_x2 = pattern_x2 * cos_theta - pattern_y2 * sin_theta;
            let rotated_y2 = pattern_x2 * sin_theta + pattern_y2 * cos_theta;
            
            // 计算在图像上的绝对坐标
            let x1 = (kp.x as f32 + rotated_x1).round() as i32;
            let y1 = (kp.y as f32 + rotated_y1).round() as i32;
            let x2 = (kp.x as f32 + rotated_x2).round() as i32;
            let y2 = (kp.y as f32 + rotated_y2).round() as i32;
            
            // 检查边界
            if x1 >= 0 && x1 < width as i32 && y1 >= 0 && y1 < height as i32 &&
               x2 >= 0 && x2 < width as i32 && y2 >= 0 && y2 < height as i32 {
                // 比较两个点的像素值
                let val1 = img.get_pixel(x1 as u32, y1 as u32)[0];
                let val2 = img.get_pixel(x2 as u32, y2 as u32)[0];
                
                // 设置描述子位
                if val1 < val2 {
                    descriptor.data[i / 8] |= 1 << (i % 8);
                }
            }
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
    // 实际实现中可以使用随机生成的点对或从高斯分布中采样
    let s = 15.0; // 采样区域大小
    
    // 预定义256个点对
    for i in 0..BRIEF_PATTERN_SIZE {
        // 简单方法: 在指定区域内生成点对
        // 实际应用中可能需要更精心设计的采样模式
        let x1 = (((i * 4) % 31) as f32 - 15.0) * s / 30.0;
        let y1 = (((i * 4 + 1) % 31) as f32 - 15.0) * s / 30.0;
        let x2 = (((i * 4 + 2) % 31) as f32 - 15.0) * s / 30.0;
        let y2 = (((i * 4 + 3) % 31) as f32 - 15.0) * s / 30.0;
        
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
    let ratio_threshold = 0.8; // Lowe's比率测试阈值
    
    // 对于描述子1中的每个描述子
    for (i, desc1) in descriptors1.iter().enumerate() {
        let mut best_distance = u32::MAX;
        let mut second_best = u32::MAX;
        let mut best_idx = 0;
        
        // 查找最佳匹配
        for (j, desc2) in descriptors2.iter().enumerate() {
            let distance = compute_hamming_distance(&desc1.data, &desc2.data);
            
            if distance < best_distance {
                second_best = best_distance;
                best_distance = distance;
                best_idx = j;
            } else if distance < second_best {
                second_best = distance;
            }
        }
        
        // 应用Lowe's比率测试，过滤掉不明确的匹配
        if best_distance < 80 && (second_best == u32::MAX || 
                                (best_distance as f32 / second_best as f32) < ratio_threshold) {
            matches.push((i, best_idx));
        }
    }
    
    matches
}

/// 计算两个描述子的汉明距离
fn compute_hamming_distance(a: &[u8; 32], b: &[u8; 32]) -> u32 {
    let mut distance = 0;
    
    for i in 0..32 {
        let xor = a[i] ^ b[i];
        // 计算设置的位数
        distance += xor.count_ones();
    }
    
    distance
}