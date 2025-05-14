/**
 * Tauri API 封装模块
 * 提供与 Tauri 后端通信的统一接口
 */

import { invoke } from "@tauri-apps/api/tauri";
import { open } from "@tauri-apps/api/dialog";
import { open as openPath } from "@tauri-apps/api/shell";

/**
 * 选择文件夹对话框
 * @param {Object} options - 选项
 * @param {boolean} options.multiple - 是否允许多选
 * @param {string} options.title - 对话框标题
 * @returns {Promise<string|string[]|null>} - 选择的文件夹路径或路径数组
 */
export async function selectFolders({
  multiple = true,
  title = "选择文件夹",
} = {}) {
  try {
    return await open({
      directory: true,
      multiple,
      title,
    });
  } catch (error) {
    console.error("选择文件夹失败:", error);
    throw error;
  }
}

/**
 * 查找重复图片
 * @param {Object} options - 查找选项
 * @param {string[]} options.folderPaths - 要扫描的文件夹路径数组
 * @param {string} options.algorithm - 使用的算法（'Exact', 'Average', 'Difference', 'Perceptual', 'Wavelet', 'ORB'）
 * @param {number} options.similarityThreshold - 相似度阈值（0-100）
 * @returns {Promise<Array>} - 重复图片组
 */
export async function findDuplicates({
  folderPaths,
  algorithm,
  similarityThreshold,
}) {
  try {
    return await invoke("find_duplicates", {
      folderPaths,
      algorithm,
      similarityThreshold: Number(similarityThreshold),
    });
  } catch (error) {
    console.error("查找重复图片失败:", error);
    throw error;
  }
}

/**
 * 打开图片文件
 * @param {string} path - 图片文件路径
 * @returns {Promise<void>}
 */
export async function openImage(path) {
  try {
    await openPath(path);
  } catch (error) {
    console.error("打开图片失败:", error);
    throw error;
  }
}

/**
 * 删除文件
 * @param {string} path - 文件路径
 * @returns {Promise<boolean>} - 删除是否成功
 */
export async function deleteFile(path) {
  try {
    return await invoke("delete_file", { path });
  } catch (error) {
    console.error("删除文件失败:", error);
    throw error;
  }
}

/**
 * 批量删除文件
 * @param {string[]} paths - 文件路径数组
 * @returns {Promise<{success: string[], failed: string[]}>} - 删除成功和失败的文件路径
 */
export async function deleteFiles(paths) {
  try {
    return await invoke("delete_files", { paths });
  } catch (error) {
    console.error("批量删除文件失败:", error);
    throw error;
  }
}

/**
 * 确认对话框
 * @param {string} message - 提示消息
 * @param {string} title - 对话框标题
 * @returns {Promise<boolean>} - 用户是否确认
 */
export async function confirm(message, title = "确认") {
  try {
    return await window.__TAURI__.dialog.confirm(message, title);
  } catch (error) {
    console.error("确认对话框显示失败:", error);
    return false;
  }
}
