use std::path::{Path, PathBuf};
use std::fs;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
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
    
    // 4. 按组大小排序，最大的组在最前面
    let mut sorted_groups = duplicate_groups;
    sorted_groups.sort_by(|a, b| b.images.len().cmp(&a.images.len()));
    
    Ok(sorted_groups)
}

/// 并行计算所有图像的哈希值
fn compute_image_hashes(
    paths: &[PathBuf],
    algorithm: HashAlgorithm
) -> Result<Vec<HashResult>, String> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    
    // 批量处理提高性能
    const BATCH_SIZE: usize = 500;
    
    // 创建固定大小的结果向量，初始化为None
    let results = Arc::new(Mutex::new(vec![None; paths.len()]));
    let error_count = Arc::new(Mutex::new(0));
    
    // 分批并行处理
    paths.chunks(BATCH_SIZE).par_bridge().for_each(|batch| {
        let batch_results: Vec<(usize, Result<HashResult, String>)> = batch.par_iter().enumerate()
            .map(|(local_idx, path)| {
                // 计算哈希并记录原始索引
                let global_idx = local_idx + 
                    (batch.as_ptr() as usize - paths.as_ptr() as usize) / std::mem::size_of::<PathBuf>();
                
                (global_idx, algorithms::calculate_hash(path, algorithm))
            })
            .collect();
        
        // 合并批次结果
        let mut results_lock = results.lock().unwrap();
        let mut error_lock = error_count.lock().unwrap();
        
        for (idx, result) in batch_results {
            match result {
                Ok(hash) => {
                    results_lock[idx] = Some(hash);
                },
                Err(e) => {
                    *error_lock += 1;
                    eprintln!("处理图像失败 {}: {}", paths[idx].display(), e);
                }
            }
        }
    });
    
    // 获取最终结果
    let final_results = Arc::try_unwrap(results)
        .expect("无法获取锁")
        .into_inner()
        .expect("锁被毒化");
    
    let final_error_count = *error_count.lock().unwrap();
    
    // 将Option<HashResult>转换为HashResult，对于None的情况使用空哈希值
    let valid_hashes: Vec<HashResult> = final_results.into_iter()
        .map(|opt_result| opt_result.unwrap_or_else(|| HashResult {
            hash: String::new(),
            width: 0,
            height: 0,
        }))
        .collect();
    
    if final_error_count > 0 {
        eprintln!("注意: {} 个图像处理失败", final_error_count);
    }
    
    if valid_hashes.is_empty() {
        Err("所有图像处理均失败".to_string())
    } else {
        Ok(valid_hashes)
    }
}

/// 寻找重复图像并分组
fn find_duplicate_groups(
    paths: &[PathBuf],
    hashes: &[HashResult],
    algorithm: HashAlgorithm,
    threshold: f32
) -> Result<Vec<DuplicateGroup>, String> {
    if hashes.is_empty() {
        return Ok(Vec::new());
    }
    
    if paths.len() != hashes.len() {
        return Err(format!("哈希值({})与路径({})数量不匹配", hashes.len(), paths.len()));
    }
    
    // 提取所有哈希字符串用于LSH算法
    let hash_strings: Vec<String> = hashes.iter().map(|h| h.hash.clone()).collect();
    
    // 使用LSH算法快速找到可能的候选对
    let candidate_pairs = compute_candidate_pairs(&hash_strings, algorithm);
    
    // 并行计算所有候选对的相似度
    let similarity_results: Vec<((usize, usize), f32)> = candidate_pairs
        .par_iter()
        .map(|&(i, j)| {
            let hash1 = &hash_strings[i];
            let hash2 = &hash_strings[j];
            let similarity = algorithms::calculate_similarity(hash1, hash2, algorithm);
            ((i, j), similarity)
        })
        .filter(|(_, similarity)| *similarity >= threshold)
        .collect();
    
    // 使用并查集算法构建连通分量（相似图像组）
    let mut disjoint_set = DisjointSet::new(hashes.len());
    
    // 合并相似的图像对
    for ((i, j), _) in &similarity_results {
        disjoint_set.union(*i, *j);
    }
    
    // 从并查集构建组
    let mut group_map: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..hashes.len() {
        let root = disjoint_set.find(i);
        group_map.entry(root).or_insert_with(Vec::new).push(i);
    }
    
    // 过滤并构建最终的重复组
    let mut groups = Vec::new();
    
    for (_, indices) in group_map.iter() {
        // 只处理大于1的组（实际重复）
        if indices.len() <= 1 {
            continue;
        }
        
        // 收集组内所有图像信息
        let images: Vec<ImageInfo> = indices.par_iter()
            .filter_map(|&idx| {
                let path = &paths[idx];
                let hash_result = &hashes[idx];
                
                match get_file_metadata(path) {
                    Ok((size_bytes, created_at, modified_at)) => {
                        Some(ImageInfo {
                            path: path.to_string_lossy().into_owned(),
                            hash: hash_result.hash.clone(),
                            width: hash_result.width,
                            height: hash_result.height,
                            size_bytes,
                            created_at,
                            modified_at,
                        })
                    },
                    Err(_) => None
                }
            })
            .collect();
        
        // 如果组内有多个有效图像，添加到结果中
        if images.len() > 1 {
            groups.push(DuplicateGroup {
                images,
                similarity_threshold: threshold,
            });
        }
    }
    
    Ok(groups)
}

/// 并查集数据结构，用于高效地构建连通分量
struct DisjointSet {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl DisjointSet {
    fn new(size: usize) -> Self {
        let mut parent = Vec::with_capacity(size);
        let rank = vec![0; size];
        
        // 初始化，每个元素都是自己的父节点
        for i in 0..size {
            parent.push(i);
        }
        
        Self { parent, rank }
    }
    
    /// 查找元素所属的集合代表
    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            // 路径压缩：将x的父节点直接设为根节点
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }
    
    /// 合并两个元素所在的集合
    fn union(&mut self, x: usize, y: usize) {
        let root_x = self.find(x);
        let root_y = self.find(y);
        
        if root_x == root_y {
            return; // 已经在同一集合中
        }
        
        // 按秩合并：将秩较小的树连接到秩较大的树上
        if self.rank[root_x] < self.rank[root_y] {
            self.parent[root_x] = root_y;
        } else if self.rank[root_x] > self.rank[root_y] {
            self.parent[root_y] = root_x;
        } else {
            // 秩相同，任意方向合并，并增加秩
            self.parent[root_y] = root_x;
            self.rank[root_x] += 1;
        }
    }
}

/// 检查两张图片是否可能是重复的
/// 允许使用更严格的过滤条件
pub fn are_images_duplicates(
    img1_path: &Path,
    img2_path: &Path,
    algorithm: HashAlgorithm,
    threshold: f32
) -> Result<bool, String> {
    // 快速检查：如果是同一个文件，直接返回true
    if img1_path.canonicalize().ok() == img2_path.canonicalize().ok() {
        return Ok(true);
    }
    
    // 检查文件大小（可选的快速过滤）
    if let (Ok(metadata1), Ok(metadata2)) = (fs::metadata(img1_path), fs::metadata(img2_path)) {
        // 如果文件大小差异超过50%，则不太可能是重复的（可选性能优化）
        if metadata1.len() > 0 && metadata2.len() > 0 {
            let size_ratio = if metadata1.len() > metadata2.len() {
                metadata1.len() as f64 / metadata2.len() as f64
            } else {
                metadata2.len() as f64 / metadata1.len() as f64
            };
            
            // 对于精确匹配算法，文件大小必须完全相同
            if algorithm == HashAlgorithm::Exact && metadata1.len() != metadata2.len() {
                return Ok(false);
            }
            
            // 对于其他算法，如果大小差异过大，可能不是重复的
            if algorithm != HashAlgorithm::Exact && size_ratio > 2.0 {
                return Ok(false);
            }
        }
    }
    
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