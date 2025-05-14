import { ref, reactive } from 'vue';
import { findDuplicates } from '../utils/tauri';

/**
 * 图片处理相关的可复用逻辑
 * @param {Object} options - 配置选项
 * @returns {Object} - 图片处理相关的状态和方法
 */
export function useImageProcessing(options = {}) {
  // 处理状态
  const processing = ref(false);
  const status = ref('');
  const progress = ref(0);
  const error = ref(null);
  
  // 结果状态
  const results = reactive({
    duplicateGroups: [],
    totalImages: 0,
    totalGroups: 0,
    totalDuplicates: 0,
    algorithm: '',
    similarityThreshold: 0,
    processingTime: 0,
    timestamp: null
  });

  /**
   * 开始处理图片
   * @param {Object} params - 处理参数
   * @param {string[]} params.folderPaths - 文件夹路径数组
   * @param {string} params.algorithm - 使用的算法
   * @param {number} params.similarityThreshold - 相似度阈值
   * @returns {Promise<Object>} - 处理结果
   */
  const startProcessing = async ({ folderPaths, algorithm, similarityThreshold }) => {
    if (!folderPaths?.length) {
      status.value = '请选择至少一个文件夹';
      return null;
    }

    try {
      // 重置状态
      processing.value = true;
      status.value = '正在扫描图片...';
      progress.value = 0;
      error.value = null;
      
      const startTime = Date.now();
      
      // 查找重复图片
      const duplicateGroups = await findDuplicates({
        folderPaths,
        algorithm,
        similarityThreshold
      });
      
      const endTime = Date.now();
      
      // 更新结果
      results.duplicateGroups = processGroupsData(duplicateGroups);
      results.totalGroups = results.duplicateGroups.length;
      results.totalImages = getTotalImages(results.duplicateGroups);
      results.totalDuplicates = getTotalDuplicates(results.duplicateGroups);
      results.algorithm = algorithm;
      results.similarityThreshold = similarityThreshold;
      results.processingTime = endTime - startTime;
      results.timestamp = new Date().toISOString();
      
      status.value = `处理完成，共找到 ${results.totalDuplicates} 张重复图片，分为 ${results.totalGroups} 组`;
      progress.value = 100;
      
      return {
        ...results,
        success: true
      };
    } catch (err) {
      error.value = err;
      status.value = `处理出错: ${err.message || err}`;
      return {
        success: false,
        error: err
      };
    } finally {
      processing.value = false;
    }
  };

  /**
   * 处理分组数据，添加额外的元数据
   * @param {Array} groups - 原始分组数据
   * @returns {Array} - 处理后的分组数据
   */
  const processGroupsData = (groups) => {
    return groups.map((group, index) => {
      // 计算每组的关键指标
      const totalSize = group.images.reduce((sum, img) => sum + (img.size_bytes || 0), 0);
      const avgSize = totalSize / group.images.length;
      
      // 标记最佳质量的图片（默认保留第一张）
      const bestQualityIndex = getBestQualityImageIndex(group.images);
      
      return {
        ...group,
        id: `group-${index}`,
        totalSize,
        avgSize,
        bestQualityIndex,
        // 为每张图片添加额外信息
        images: group.images.map((img, imgIndex) => ({
          ...img,
          id: `img-${index}-${imgIndex}`,
          isBest: imgIndex === bestQualityIndex,
          sizeDiff: Math.abs((img.size_bytes || 0) - avgSize)
        }))
      };
    });
  };

  /**
   * 确定最佳质量图片的索引
   * @param {Array} images - 图片数组
   * @returns {number} - 最佳质量图片的索引
   */
  const getBestQualityImageIndex = (images) => {
    // 默认选择第一张图片
    if (!images || images.length === 0) return 0;
    
    // 简单算法：选择分辨率最高、文件大小适中的图片
    let bestIndex = 0;
    let bestScore = 0;
    
    images.forEach((img, index) => {
      const resolution = (img.width || 0) * (img.height || 0);
      const size = img.size_bytes || 0;
      
      // 计算质量得分（分辨率更重要，但文件大小也有影响）
      // 过大的文件可能是未压缩的或包含不必要的元数据
      const sizeWeight = size > 10 * 1024 * 1024 ? 0.7 : 1; // 大于10MB的文件权重降低
      const score = resolution * sizeWeight;
      
      if (score > bestScore) {
        bestScore = score;
        bestIndex = index;
      }
    });
    
    return bestIndex;
  };

  /**
   * 计算总图片数
   * @param {Array} groups - 图片分组
   * @returns {number} - 总图片数
   */
  const getTotalImages = (groups) => {
    return groups.reduce((sum, group) => sum + (group.images?.length || 0), 0);
  };

  /**
   * 计算重复图片数（总图片数减去每组保留的一张）
   * @param {Array} groups - 图片分组
   * @returns {number} - 重复图片数
   */
  const getTotalDuplicates = (groups) => {
    return getTotalImages(groups) - groups.length;
  };

  /**
   * 重置处理状态
   */
  const reset = () => {
    processing.value = false;
    status.value = '';
    progress.value = 0;
    error.value = null;
    
    // 重置结果
    results.duplicateGroups = [];
    results.totalImages = 0;
    results.totalGroups = 0;
    results.totalDuplicates = 0;
    results.algorithm = '';
    results.similarityThreshold = 0;
    results.processingTime = 0;
    results.timestamp = null;
  };

  return {
    // 状态
    processing,
    status,
    progress,
    error,
    results,
    
    // 方法
    startProcessing,
    reset
  };
}