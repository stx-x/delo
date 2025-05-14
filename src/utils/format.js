/**
 * 格式化文件大小
 * @param {number} bytes - 文件大小（字节）
 * @param {number} decimals - 小数位数
 * @returns {string} 格式化后的文件大小字符串
 */
export function formatFileSize(bytes, decimals = 2) {
  if (bytes === 0) return '0 B';
  
  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB'];
  
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  
  return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
}

/**
 * 格式化日期时间
 * @param {Date|string|number} date - 日期对象、时间戳或日期字符串
 * @param {string} format - 格式化模板，默认为 'YYYY-MM-DD HH:mm:ss'
 * @returns {string} 格式化后的日期字符串
 */
export function formatDateTime(date, format = 'YYYY-MM-DD HH:mm:ss') {
  const d = new Date(date);
  
  if (isNaN(d.getTime())) {
    return '无效日期';
  }
  
  const formatMap = {
    YYYY: d.getFullYear().toString(),
    MM: (d.getMonth() + 1).toString().padStart(2, '0'),
    DD: d.getDate().toString().padStart(2, '0'),
    HH: d.getHours().toString().padStart(2, '0'),
    mm: d.getMinutes().toString().padStart(2, '0'),
    ss: d.getSeconds().toString().padStart(2, '0'),
    YY: d.getFullYear().toString().slice(-2),
    M: (d.getMonth() + 1).toString(),
    D: d.getDate().toString(),
    H: d.getHours().toString(),
    m: d.getMinutes().toString(),
    s: d.getSeconds().toString()
  };
  
  return format.replace(/YYYY|MM|DD|HH|mm|ss|YY|M|D|H|m|s/g, match => formatMap[match]);
}

/**
 * 格式化相对时间（如：刚刚、1分钟前等）
 * @param {Date|string|number} date - 日期对象、时间戳或日期字符串
 * @returns {string} 相对时间字符串
 */
export function formatRelativeTime(date) {
  const now = new Date();
  const d = new Date(date);
  
  if (isNaN(d.getTime())) {
    return '无效日期';
  }
  
  const diff = Math.floor((now - d) / 1000); // 秒差
  
  if (diff < 30) return '刚刚';
  if (diff < 60) return `${diff}秒前`;
  
  const minutes = Math.floor(diff / 60);
  if (minutes < 60) return `${minutes}分钟前`;
  
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}小时前`;
  
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}天前`;
  
  const months = Math.floor(days / 30);
  if (months < 12) return `${months}个月前`;
  
  const years = Math.floor(months / 12);
  return `${years}年前`;
}

/**
 * 格式化文件路径（截断过长路径）
 * @param {string} path - 完整文件路径
 * @param {number} maxLength - 最大显示长度
 * @returns {string} 格式化后的路径
 */
export function formatPath(path, maxLength = 40) {
  if (!path) return '';
  if (path.length <= maxLength) return path;
  
  const parts = path.split(/[/\\]/);
  
  // 如果只有一个部分或路径不够长，则直接返回
  if (parts.length <= 2) {
    return path.slice(-maxLength).padStart(maxLength + 3, '...');
  }
  
  // 保留最后两个部分，其他部分用省略号替代
  const filename = parts.pop();
  const parentDir = parts.pop();
  return `.../${parentDir}/${filename}`;
}