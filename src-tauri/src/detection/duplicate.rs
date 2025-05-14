use crate::algorithms;
use crate::core::types::{DuplicateGroup, HashAlgorithm, HashResult, ImageInfo};
use crate::core::utils::file_utils::{get_file_metadata, get_image_paths};
use crate::detection::lsh::{compute_candidate_pairs, LSHIndex};
use bit_vec::BitVec;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

// 缓存，用于存储已计算的相似度，避免重复计算
static mut SIMILARITY_CACHE: Option<HashMap<String, f32>> = None;

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
    let start_time = Instant::now();

    // 初始化或清空相似度缓存
    unsafe {
        if SIMILARITY_CACHE.is_none() {
            SIMILARITY_CACHE = Some(HashMap::with_capacity(10000));
        } else {
            SIMILARITY_CACHE.as_mut().unwrap().clear();
        }
    }

    // 1. 收集所有图像路径
    let mut all_image_paths = Vec::new();

    // 并行处理文件夹
    let folder_paths: Vec<_> = params
        .folders
        .par_iter()
        .map(|folder| -> Result<Vec<PathBuf>, String> { get_image_paths(folder, params.recursive) })
        .collect::<Result<Vec<_>, String>>()?;

    for paths in folder_paths {
        all_image_paths.extend(paths);
    }

    if all_image_paths.is_empty() {
        return Ok(Vec::new());
    }

    println!(
        "收集完成 {} 张图片，耗时: {:?}",
        all_image_paths.len(),
        start_time.elapsed()
    );
    let hash_start = Instant::now();

    // 2. 计算所有图像的哈希值
    let image_hashes = compute_image_hashes(&all_image_paths, params.algorithm)?;

    println!("哈希计算完成，耗时: {:?}", hash_start.elapsed());
    let group_start = Instant::now();

    // 3. 根据哈希值找出重复图像
    let duplicate_groups = find_duplicate_groups(
        &all_image_paths,
        &image_hashes,
        params.algorithm,
        params.threshold,
    )?;

    println!(
        "分组完成，找到 {} 组重复，耗时: {:?}",
        duplicate_groups.len(),
        group_start.elapsed()
    );

    // 4. 按组大小排序，最大的组在最前面
    let mut sorted_groups = duplicate_groups;
    sorted_groups.sort_by(|a, b| b.images.len().cmp(&a.images.len()));

    println!("总耗时: {:?}", start_time.elapsed());

    // 清理缓存
    unsafe {
        if let Some(cache) = SIMILARITY_CACHE.as_mut() {
            cache.clear();
        }
    }

    Ok(sorted_groups)
}

/// 并行计算所有图像的哈希值，使用改进的内存管理和并行策略
fn compute_image_hashes(
    paths: &[PathBuf],
    algorithm: HashAlgorithm,
) -> Result<Vec<HashResult>, String> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }

    // 批量处理提高性能
    const BATCH_SIZE: usize = 500;

    // 创建线程安全的结果容器
    let results = Arc::new(parking_lot::Mutex::new(
        Vec::<Option<HashResult>>::with_capacity(paths.len()),
    ));
    results.lock().resize_with(paths.len(), || None);

    let error_count = Arc::new(parking_lot::Mutex::new(0));

    // 并行处理所有路径
    paths.par_chunks(BATCH_SIZE).for_each(|batch| {
        // 为每个批次创建本地结果容器
        let batch_results: Vec<(usize, Result<HashResult, String>)> = batch
            .par_iter()
            .enumerate()
            .map(|(local_idx, path)| {
                // 计算全局索引
                let path_ptr = path as *const PathBuf as usize;
                let base_ptr = paths.as_ptr() as usize;
                let offset = (path_ptr - base_ptr) / std::mem::size_of::<PathBuf>();

                // 计算哈希
                (offset, algorithms::calculate_hash(path, algorithm))
            })
            .collect();

        // 获取锁并更新结果
        let mut results_lock = results.lock();
        let mut err_count = error_count.lock();

        for (idx, result) in batch_results {
            match result {
                Ok(hash) => {
                    if idx < results_lock.len() {
                        results_lock[idx] = Some(hash);
                    }
                }
                Err(e) => {
                    *err_count += 1;
                    if idx < paths.len() {
                        eprintln!("处理图像失败 {}: {}", paths[idx].display(), e);
                    }
                }
            }
        }
    });

    // 获取最终结果并过滤None值
    let final_results = Arc::try_unwrap(results).expect("无法获取锁").into_inner();

    let final_error_count = *error_count.lock();

    // 过滤掉None值（处理失败的图像）
    let valid_hashes: Vec<HashResult> = final_results.into_iter().filter_map(|opt| opt).collect();

    if !valid_hashes.is_empty() {
        if final_error_count > 0 {
            eprintln!("注意: {} 个图像处理失败，已忽略", final_error_count);
        }
        Ok(valid_hashes)
    } else {
        Err("所有图像处理均失败".to_string())
    }
}

/// 寻找重复图像并分组
fn find_duplicate_groups(
    paths: &[PathBuf],
    hashes: &[HashResult],
    algorithm: HashAlgorithm,
    threshold: f32,
) -> Result<Vec<DuplicateGroup>, String> {
    if hashes.is_empty() {
        return Ok(Vec::new());
    }

    if paths.len() != hashes.len() {
        return Err(format!(
            "哈希值({})与路径({})数量不匹配",
            hashes.len(),
            paths.len()
        ));
    }

    // 提取所有哈希字符串用于LSH算法
    let hash_strings: Vec<String> = hashes.iter().map(|h| h.hash.clone()).collect();

    // 使用LSH算法快速找到可能的候选对
    let candidate_pairs = compute_candidate_pairs(&hash_strings, algorithm);

    println!("LSH找到 {} 个候选对", candidate_pairs.len());

    // 创建位向量缓存，用于加速相似度计算
    let bit_vectors: Vec<Option<BitVec>> = hash_strings
        .iter()
        .map(|hash| {
            if hash.len() >= 64 && hash.chars().all(|c| c == '0' || c == '1') {
                // 对于二进制哈希，转换为BitVec
                let mut bv = BitVec::with_capacity(hash.len());
                for c in hash.chars() {
                    bv.push(c == '1');
                }
                Some(bv)
            } else {
                None
            }
        })
        .collect();

    // 并行计算所有候选对的相似度，使用缓存避免重复计算
    let similarity_results: Vec<((usize, usize), f32)> = candidate_pairs
        .par_iter()
        .map(|&(i, j)| {
            let hash1 = &hash_strings[i];
            let hash2 = &hash_strings[j];

            // 使用缓存
            let cache_key = if hash1 < hash2 {
                format!("{}:{}", i, j)
            } else {
                format!("{}:{}", j, i)
            };

            // 检查缓存
            let similarity = unsafe {
                if let Some(cache) = &SIMILARITY_CACHE {
                    if let Some(sim) = cache.get(&cache_key) {
                        *sim
                    } else {
                        // 优化相似度计算：使用位向量加速
                        let sim = match (&bit_vectors[i], &bit_vectors[j]) {
                            (Some(bv1), Some(bv2)) if bv1.len() == bv2.len() => {
                                // 使用位向量计算汉明距离
                                let mut distance = 0;
                                for k in 0..bv1.len() {
                                    if bv1[k] != bv2[k] {
                                        distance += 1;
                                    }
                                }
                                100.0 * (1.0 - (distance as f32 / bv1.len() as f32))
                            }
                            _ => algorithms::calculate_similarity(hash1, hash2, algorithm),
                        };

                        // 更新缓存
                        if let Some(cache) = &mut SIMILARITY_CACHE {
                            cache.insert(cache_key, sim);
                        }

                        sim
                    }
                } else {
                    algorithms::calculate_similarity(hash1, hash2, algorithm)
                }
            };

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
        let images: Vec<ImageInfo> = indices
            .par_iter()
            .filter_map(|&idx| {
                let path = &paths[idx];
                let hash_result = &hashes[idx];

                match get_file_metadata(path) {
                    Ok((size_bytes, created_at, modified_at)) => Some(ImageInfo {
                        path: path.to_string_lossy().into_owned(),
                        hash: hash_result.hash.clone(),
                        width: hash_result.width,
                        height: hash_result.height,
                        size_bytes,
                        created_at,
                        modified_at,
                    }),
                    Err(_) => None,
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
/// 优化的实现，减少内存访问次数
struct DisjointSet {
    parent: Vec<usize>,
    rank: Vec<u8>, // 使用更小的数据类型，提高缓存效率
    // 新增：size数组跟踪每个集合的大小，用于优化合并
    size: Vec<usize>,
}

impl DisjointSet {
    fn new(size: usize) -> Self {
        let mut parent = Vec::with_capacity(size);
        // 使用u8类型存储秩，减少内存使用
        let rank = vec![0u8; size];

        // 初始化，每个元素都是自己的父节点
        for i in 0..size {
            parent.push(i);
        }

        // 初始化每个集合的大小为1
        let size_vec = vec![1; size];

        Self {
            parent,
            rank,
            size: size_vec,
        }
    }

    /// 查找元素所属的集合代表，使用路径压缩优化
    fn find(&mut self, mut x: usize) -> usize {
        // 非递归实现路径压缩，减少栈使用并提高性能
        let mut root = x;

        // 找到根节点
        while self.parent[root] != root {
            root = self.parent[root];
        }

        // 路径压缩：将路径上的所有节点直接连接到根节点
        while x != root {
            let next = self.parent[x];
            self.parent[x] = root;
            x = next;
        }

        root
    }

    /// 合并两个元素所在的集合，使用按秩合并和按大小合并
    fn union(&mut self, x: usize, y: usize) {
        let mut root_x = self.find(x);
        let mut root_y = self.find(y);

        if root_x == root_y {
            return; // 已经在同一集合中
        }

        // 按大小和秩合并：确保较小的树总是合并到较大的树上
        if self.size[root_x] < self.size[root_y] {
            std::mem::swap(&mut root_x, &mut root_y);
        }

        // 合并树
        self.parent[root_y] = root_x;
        self.size[root_x] += self.size[root_y];

        // 按秩合并的逻辑
        if self.rank[root_x] == self.rank[root_y] {
            self.rank[root_x] += 1;
        }
    }

    /// 获取集合大小
    fn get_size(&self, x: usize) -> usize {
        self.size[x]
    }
}

/// 检查两张图片是否可能是重复的
/// 允许使用更严格的过滤条件
pub fn are_images_duplicates(
    img1_path: &Path,
    img2_path: &Path,
    algorithm: HashAlgorithm,
    threshold: f32,
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
pub fn get_all_image_paths(folders: &[PathBuf], recursive: bool) -> Result<Vec<PathBuf>, String> {
    // 使用并行处理，加速大量文件夹的扫描
    let folder_paths: Vec<Result<Vec<PathBuf>, String>> = folders
        .par_iter()
        .map(|folder| get_image_paths(folder, recursive))
        .collect();

    // 合并结果
    let mut all_paths = Vec::new();
    for result in folder_paths {
        match result {
            Ok(paths) => all_paths.extend(paths),
            Err(e) => return Err(e),
        }
    }

    Ok(all_paths)
}
