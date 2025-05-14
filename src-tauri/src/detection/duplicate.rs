use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use rayon::prelude::*;
use crate::core::types::{HashAlgorithm, HashResult, DuplicateGroup, ImageInfo};
use crate::core::utils::file_utils::{get_image_paths, get_file_metadata};
use crate::algorithms;
use crate::detection::lsh::{LSHIndex, compute_candidate_pairs};

/// 检测重复图像请求参数
#[derive(Debug, Clone)]
pub struct DuplicateDetectionParams {
    /// 文件夹路径列表
    pub folders: Vec<PathBuf>,
    /// 哈希算法
    pub algorithm: HashAlgorithm,
    /// 相似度阈值 (0-100)
    pub threshold: f32,
    /// 是否递归子文件夹
    pub recursive: bool,
}

/// 执行重复图像检测
pub fn detect_duplicates(params: &DuplicateDetectionParams) -> Result<Vec<DuplicateGroup>, String> {
    // 1. 收集所有图像路径
    let mut all_image_paths = Vec::new();
    
    for folder in &params.folders {
        let mut paths = get_image_paths(folder, params.recursive)?;
        all_image_paths.append(&mut paths);
    }
    
    if all_image_paths.is_empty() {
        return Ok(Vec::new());
    }
    
    // 2. 计算所有图像的哈希值
    let image_hashes = compute_image_hashes(&all_image_paths, params.algorithm)?;
    
    // 3. 根据哈希值找出重复图像
    let duplicate_groups = find_duplicate_groups(
        &all_image_paths,
        &image_hashes,
        params.algorithm,
        params.threshold
    )?;
    
    Ok(duplicate_groups)
}

/// 并行计算所有图像的哈希值
fn compute_image_hashes(
    paths: &[PathBuf],
    algorithm: HashAlgorithm
) -> Result<Vec<HashResult>, String> {
    // 使用Rayon并行计算哈希值
    let results: Vec<Result<HashResult, String>> = paths.par_iter()
        .map(|path| algorithms::calculate_hash(path, algorithm))
        .collect();
    
    // 过滤出成功的结果
    let mut hashes = Vec::with_capacity(paths.len());
    let mut error_count = 0;
    
    for (i, result) in results.into_iter().enumerate() {
        match result {
            Ok(hash) => hashes.push(hash),
            Err(e) => {
                error_count += 1;
                eprintln!("处理图像失败 {}: {}", paths[i].display(), e);
            }
        }
    }
    
    if !hashes.is_empty() {
        if error_count > 0 {
            eprintln!("注意: {} 个图像处理失败，已忽略", error_count);
        }
        Ok(hashes)
    } else {
        Err("所有图像处理均失败".to_string())
    }
}

/// 寻找重复图像并分组
fn find_duplicate_groups(
    paths: &[PathBuf],
    hashes: &[HashResult],
    algorithm: HashAlgorithm,
    threshold: f32
) -> Result<Vec<DuplicateGroup>, String> {
    if hashes.is_empty() || paths.len() != hashes.len() {
        return Err("哈希值与路径数量不匹配".to_string());
    }
    
    let mut groups = Vec::new();
    let mut used = vec![false; hashes.len()];
    
    // 提取所有哈希字符串用于LSH算法
    let hash_strings: Vec<String> = hashes.iter().map(|h| h.hash.clone()).collect();
    
    // 使用LSH算法快速找到可能的候选对
    let candidate_pairs = compute_candidate_pairs(&hash_strings, algorithm);
    
    // 构建每个图像的候选集
    let mut similarity_cache = HashMap::new();
    let mut candidate_map: HashMap<usize, Vec<usize>> = HashMap::new();
    
    for (i, j) in candidate_pairs {
        // 计算相似度
        let hash1 = &hash_strings[i];
        let hash2 = &hash_strings[j];
        
        let cache_key = if hash1 < hash2 {
            format!("{}{}", hash1, hash2)
        } else {
            format!("{}{}", hash2, hash1)
        };
        
        let similarity = if let Some(&sim) = similarity_cache.get(&cache_key) {
            sim
        } else {
            let sim = algorithms::calculate_similarity(hash1, hash2, algorithm);
            similarity_cache.insert(cache_key, sim);
            sim
        };
        
        // 如果相似度高于阈值，添加到候选集
        if similarity >= threshold {
            candidate_map.entry(i).or_insert_with(Vec::new).push(j);
            candidate_map.entry(j).or_insert_with(Vec::new).push(i);
        }
    }
    
    // 构建重复图像组
    for i in 0..hashes.len() {
        if used[i] {
            continue;
        }
        
        // 获取第i个图像的所有候选匹配
        let candidates = match candidate_map.get(&i) {
            Some(c) => c,
            None => continue, // 没有候选，继续下一个
        };
        
        if candidates.is_empty() {
            continue;
        }
        
        // 创建一个新组
        let mut group_indices = vec![i];
        used[i] = true;
        
        // 添加所有未使用的候选匹配
        for &j in candidates {
            if !used[j] {
                group_indices.push(j);
                used[j] = true;
            }
        }
        
        // 收集组内所有图像信息
        let mut images = Vec::with_capacity(group_indices.len());
        
        for &idx in &group_indices {
            let path = &paths[idx];
            let hash_result = &hashes[idx];
            
            if let Ok((size_bytes, created_at, modified_at)) = get_file_metadata(path) {
                images.push(ImageInfo {
                    path: path.to_string_lossy().into_owned(),
                    hash: hash_result.hash.clone(),
                    width: hash_result.width,
                    height: hash_result.height,
                    size_bytes,
                    created_at,
                    modified_at,
                });
            }
        }
        
        // 如果组内有多个图像，添加到结果中
        if images.len() > 1 {
            groups.push(DuplicateGroup {
                images,
                similarity_threshold: threshold,
            });
        }
    }
    
    // 按照组大小排序，最大组在前
    groups.sort_by(|a, b| b.images.len().cmp(&a.images.len()));
    
    Ok(groups)
}

/// 检查两张图片是否可能是重复的
/// 允许使用更严格的过滤条件
pub fn are_images_duplicates(
    img1_path: &Path,
    img2_path: &Path,
    algorithm: HashAlgorithm,
    threshold: f32
) -> Result<bool, String> {
    // 计算两张图片的哈希值
    let hash1 = algorithms::calculate_hash(img1_path, algorithm)?;
    let hash2 = algorithms::calculate_hash(img2_path, algorithm)?;
    
    // 计算相似度
    let similarity = algorithms::calculate_similarity(&hash1.hash, &hash2.hash, algorithm);
    
    Ok(similarity >= threshold)
}

/// 获取所有文件夹中的图像路径
pub fn get_all_image_paths(
    folders: &[PathBuf],
    recursive: bool
) -> Result<Vec<PathBuf>, String> {
    let mut all_paths = Vec::new();
    
    for folder in folders {
        let mut paths = get_image_paths(folder, recursive)?;
        all_paths.append(&mut paths);
    }
    
    Ok(all_paths)
}