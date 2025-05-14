use std::path::{Path, PathBuf};
use std::fs;
use walkdir::WalkDir;

/// 支持的图像格式后缀名
pub const SUPPORTED_IMAGE_EXTENSIONS: [&str; 7] = [
    "jpg", "jpeg", "png", "gif", "webp", "bmp", "tiff"
];

/// 检查文件是否是支持的图像文件
pub fn is_image_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        if let Some(ext_str) = ext.to_str() {
            return SUPPORTED_IMAGE_EXTENSIONS.contains(&ext_str.to_lowercase().as_str());
        }
    }
    false
}

/// 获取目录中的所有图像文件路径
pub fn get_image_paths(dir_path: &Path, recursive: bool) -> Result<Vec<PathBuf>, String> {
    if !dir_path.exists() {
        return Err(format!("目录不存在: {}", dir_path.display()));
    }
    
    if !dir_path.is_dir() {
        return Err(format!("路径不是一个目录: {}", dir_path.display()));
    }
    
    let mut image_paths = Vec::new();
    
    // 根据是否递归使用不同的方式遍历
    if recursive {
        // 递归遍历所有子目录
        for entry in WalkDir::new(dir_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            // 跳过符号链接
            if path.is_symlink() {
                continue;
            }
            
            if path.is_file() && is_image_file(path) {
                image_paths.push(path.to_path_buf());
            }
        }
    } else {
        // 只遍历当前目录
        if let Ok(entries) = fs::read_dir(dir_path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                // 跳过符号链接
                if path.is_symlink() {
                    continue;
                }
                
                if path.is_file() && is_image_file(&path) {
                    image_paths.push(path);
                }
            }
        }
    }
    
    Ok(image_paths)
}

/// 获取文件的元数据信息
pub fn get_file_metadata(path: &Path) -> Result<(u64, String, String), String> {
    // 检查是否是符号链接
    if path.is_symlink() {
        return Err(format!("文件是符号链接: {}", path.display()));
    }
    
    let metadata = fs::metadata(path)
        .map_err(|e| format!("无法读取文件元数据: {}", e))?;
    
    let size_bytes = metadata.len();
    
    // 创建时间
    let created_at = metadata.created()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|| "0".to_string());
    
    // 修改时间
    let modified_at = metadata.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|| "0".to_string());
    
    Ok((size_bytes, created_at, modified_at))
}