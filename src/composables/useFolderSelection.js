import { ref } from 'vue';
import { selectFolders } from '../utils/tauri';
import { formatPath } from '../utils/format';

/**
 * 文件夹选择相关的可复用逻辑
 * @returns {Object} - 文件夹选择相关的状态和方法
 */
export function useFolderSelection() {
  // 选择的文件夹列表
  const folders = ref([]);
  
  // 状态
  const status = ref('');
  const loading = ref(false);

  /**
   * 打开文件夹选择对话框
   * @param {Object} options - 选项
   * @returns {Promise<boolean>} - 选择是否成功
   */
  const openFolderDialog = async (options = {}) => {
    loading.value = true;
    status.value = '选择文件夹...';
    
    try {
      const selected = await selectFolders({
        multiple: true,
        title: '选择文件夹（按住 Ctrl/Command 键可选择多个）',
        ...options
      });
      
      if (Array.isArray(selected)) {
        const newFolders = selected.filter(folder => !folders.value.includes(folder));
        folders.value = [...folders.value, ...newFolders];
        status.value = `已添加 ${newFolders.length} 个文件夹，共选择 ${folders.value.length} 个文件夹`;
        return newFolders.length > 0;
      } else if (selected === null) {
        status.value = '已取消选择';
        return false;
      } else if (selected && !folders.value.includes(selected)) {
        folders.value = [...folders.value, selected];
        status.value = `已添加文件夹，共选择 ${folders.value.length} 个文件夹`;
        return true;
      }
      
      return false;
    } catch (err) {
      console.error('选择文件夹时出错：', err);
      status.value = `选择文件夹时出错: ${err.message || err}`;
      return false;
    } finally {
      loading.value = false;
    }
  };

  /**
   * 移除单个文件夹
   * @param {number} index - 文件夹索引
   */
  const removeFolder = (index) => {
    folders.value = folders.value.filter((_, i) => i !== index);
    status.value = folders.value.length > 0 
      ? `当前已选择 ${folders.value.length} 个文件夹`
      : '';
  };

  /**
   * 清除所有选择的文件夹
   */
  const clearFolders = () => {
    folders.value = [];
    status.value = '';
  };

  /**
   * 设置文件夹列表
   * @param {string[]} newFolders - 新的文件夹列表
   */
  const setFolders = (newFolders) => {
    folders.value = Array.isArray(newFolders) ? [...newFolders] : [];
    status.value = folders.value.length > 0 
      ? `已设置 ${folders.value.length} 个文件夹`
      : '';
  };

  /**
   * 获取格式化后的文件夹路径
   * @param {string} path - 原始路径
   * @returns {string} - 格式化后的路径
   */
  const getFormattedPath = (path) => {
    return formatPath(path);
  };

  return {
    // 状态
    folders,
    status,
    loading,
    
    // 方法
    openFolderDialog,
    removeFolder,
    clearFolders,
    setFolders,
    getFormattedPath
  };
}