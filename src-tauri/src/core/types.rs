use serde::{Serialize, Deserialize};
use std::path::PathBuf;

/// 哈希算法类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HashAlgorithm {
    /// 精确哈希 (SHA-256)
    Exact,
    /// 均值哈希 (Average Hash)
    Average,
    /// 差值哈希 (Difference Hash)
    Difference,
    /// 感知哈希 (Perceptual Hash)
    Perceptual,
    /// 定向FAST和旋转BRIEF
    ORB,
}

impl HashAlgorithm {
    /// 获取算法名称
    pub fn name(&self) -> &'static str {
        match self {
            Self::Exact => "精确哈希",
            Self::Average => "均值哈希",
            Self::Difference => "差值哈希",
            Self::Perceptual => "感知哈希",
            Self::ORB => "ORB特征",
        }
    }
    
    /// 这个算法是否基于特征点而非哈希值
    pub fn is_feature_based(&self) -> bool {
        matches!(self, Self::ORB)
    }
}

/// 图像信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    /// 图像路径
    pub path: String,
    /// 图像哈希值或特征编码
    pub hash: String,
    /// 图像宽度
    pub width: u32,
    /// 图像高度
    pub height: u32,
    /// 文件大小（字节）
    pub size_bytes: u64,
    /// 创建时间
    pub created_at: String,
    /// 修改时间
    pub modified_at: String,
}

/// 重复图像组
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    /// 组内的图像
    pub images: Vec<ImageInfo>,
    /// 相似度阈值
    pub similarity_threshold: f32,
}

/// 哈希计算结果
#[derive(Debug, Clone)]
pub struct HashResult {
    /// 哈希值或特征编码
    pub hash: String,
    /// 图像宽度
    pub width: u32,
    /// 图像高度
    pub height: u32,
}

/// 哈希计算请求
#[derive(Debug, Clone)]
pub struct HashRequest {
    /// 图像路径
    pub path: PathBuf,
    /// 哈希算法
    pub algorithm: HashAlgorithm,
}

/// 相似度计算请求
#[derive(Debug, Clone)]
pub struct SimilarityRequest {
    /// 第一个哈希值
    pub hash1: String,
    /// 第二个哈希值
    pub hash2: String,
    /// 哈希算法
    pub algorithm: HashAlgorithm,
}

/// 重复图像检测请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateDetectionRequest {
    /// 文件夹路径列表
    pub folder_paths: Vec<String>,
    /// 哈希算法
    pub algorithm: HashAlgorithm,
    /// 相似度阈值(0-100)
    pub similarity_threshold: u32,
    /// 是否递归子文件夹
    pub recursive: bool,
}