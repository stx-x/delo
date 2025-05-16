use crate::core::types::{DuplicateDetectionRequest, DuplicateGroup, HashAlgorithm};
use crate::detection::duplicate::{
    detect_duplicates, get_all_image_paths, DuplicateDetectionParams,
};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tauri::command;
use walkdir::WalkDir;

/// 获取文件夹中的图像文件路径
#[tauri::command(rename_all = "snake_case")]
pub fn get_image_paths(folder_path: String, recursive: bool) -> Result<Vec<PathBuf>, String> {
    let path = Path::new(&folder_path);

    if !path.exists() || !path.is_dir() {
        return Err(format!("无效的文件夹路径: {}", folder_path));
    }

    crate::core::utils::file_utils::get_image_paths(path, recursive)
}

/// 查找重复图像
#[tauri::command(rename_all = "snake_case")]
pub fn find_duplicates(req: DuplicateDetectionRequest) -> Result<Vec<DuplicateGroup>, String> {
    // 开始API调用计时
    let api_start_time = Instant::now();
    println!("开始处理重复图片检测请求...");
    
    // 转换参数
    let folder_paths: Vec<PathBuf> = req.folder_paths.iter().map(|p| PathBuf::from(p)).collect();

    let params = DuplicateDetectionParams {
        folders: folder_paths,
        algorithm: req.algorithm,
        threshold: req.similarity_threshold as f32,
        recursive: req.recursive,
    };

    println!("算法: {:?}, 相似度阈值: {}, 递归扫描: {}", 
             req.algorithm, req.similarity_threshold, req.recursive);

    // 执行重复检测
    let result = detect_duplicates(&params);
    
    // 计算API总耗时
    let api_total_time = api_start_time.elapsed();
    println!("API调用总耗时: {:?}", api_total_time);
    
    // 打印结果摘要
    match &result {
        Ok(groups) => {
            let total_images = groups.iter().map(|g| g.images.len()).sum::<usize>();
            let unique_images = groups.iter()
                .flat_map(|g| g.images.iter().map(|img| img.path.clone()))
                .collect::<std::collections::HashSet<_>>()
                .len();
                
            println!("检测完成，找到 {} 组重复图片，共涉及 {} 张图片 (去重后 {} 张不同图片)", 
                     groups.len(), total_images, unique_images);
        },
        Err(e) => {
            println!("检测失败: {}", e);
        }
    }
    
    result
}

/// 获取支持的算法列表
#[command]
pub fn get_supported_algorithms() -> Vec<String> {
    vec![
        "精确哈希".to_string(),
        "均值哈希".to_string(),
        "差值哈希".to_string(),
        "感知哈希".to_string(),
        "ORB特征".to_string(),
    ]
}

/// 计算重复检测的统计信息
#[tauri::command(rename_all = "snake_case")]
pub fn get_detection_stats(req: DuplicateDetectionRequest) -> Result<DetectionStats, String> {
    let folder_paths: Vec<PathBuf> = req.folder_paths.iter().map(|p| PathBuf::from(p)).collect();

    // 获取所有图像路径
    let all_paths = get_all_image_paths(&folder_paths, req.recursive)?;

    Ok(DetectionStats {
        image_count: all_paths.len(),
        folder_count: folder_paths.len(),
        algorithm: req.algorithm.name().to_string(),
        similarity_threshold: req.similarity_threshold,
    })
}

/// 重复检测任务的统计信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DetectionStats {
    /// 图像总数
    pub image_count: usize,
    /// 文件夹数量
    pub folder_count: usize,
    /// 使用的算法名称
    pub algorithm: String,
    /// 相似度阈值
    pub similarity_threshold: u32,
}

/// 文件夹统计信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FolderStats {
    /// 总文件数
    pub total_files: usize,
    /// 图像文件数
    pub image_count: usize,
    /// 文件夹数（包括子文件夹）
    pub folder_count: usize,
}

/// 获取文件夹的统计信息（文件总数、图像数等）
#[tauri::command(rename_all = "snake_case")]
pub fn get_folder_stats(folder_path: String, recursive: bool) -> Result<FolderStats, String> {
    let path = Path::new(&folder_path);

    if !path.exists() || !path.is_dir() {
        return Err(format!("无效的文件夹路径: {}", folder_path));
    }

    let mut stats = FolderStats {
        total_files: 0,
        image_count: 0,
        folder_count: 1, // 包括当前文件夹
    };

    // 如果递归，使用WalkDir遍历所有子目录和文件
    if recursive {
        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            if entry.path() == path {
                continue; // 跳过当前文件夹自身
            }
            
            if entry.path().is_dir() {
                stats.folder_count += 1;
            } else if entry.path().is_file() {
                stats.total_files += 1;
                
                // 检查是否为图像文件
                if crate::core::utils::file_utils::is_image_file(entry.path()) {
                    stats.image_count += 1;
                }
            }
        }
    } else {
        // 不递归，只遍历顶层目录
        
        // 读取目录内容
        for entry in std::fs::read_dir(path).map_err(|e| e.to_string())? {
            if let Ok(entry) = entry {
                let path = entry.path();
                
                // 跳过符号链接
                if path.is_symlink() {
                    continue;
                }
                
                if path.is_file() {
                    stats.total_files += 1;
                    
                    // 检查是否为图像文件
                    if crate::core::utils::file_utils::is_image_file(&path) {
                        stats.image_count += 1;
                    }
                } else if path.is_dir() {
                    stats.folder_count += 1;
                }
            }
        }
    }

    Ok(stats)
}
