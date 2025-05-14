<script setup>
import { ref, onMounted, computed, watch, inject, onUnmounted } from "vue";
import { useRouter, useRoute } from "vue-router";

const router = useRouter();
const route = useRoute();

// 获取全局状态
const globalState = inject("globalState");

// 状态管理
const state = ref({
    duplicateGroups: [],
    selectedFolders: [],
    processingStatus: "",
    selectedImages: {},
    showImagePreview: false,
    previewImage: null,
    currentGroupIndex: 0,
    currentImageIndex: 0,
    showBackToTop: false,
    showGroupNav: false,
    scanStats: {
        totalFolders: 0,
        totalFiles: 0,
        totalImages: 0,
        processedImages: 0,
    },
});

// 设置面板状态
const showSettings = ref(false);
const newAlgorithm = ref(globalState.algorithm || 'Exact');
const newThreshold = ref(globalState.similarityThreshold || 80);
const newFolders = ref([...globalState.selectedFolders || []]);
// 默认始终递归扫描子文件夹
const newRecursive = ref(true);
const isRescanning = ref(false);

// 算法列表
const algorithms = [
    { id: 'Exact', name: '精确哈希' },
    { id: 'Average', name: '均值哈希' },
    { id: 'Difference', name: '差值哈希' },
    { id: 'Perceptual', name: '感知哈希' },
    { id: 'ORB', name: 'ORB特征' }
];

// 打开文件夹选择对话框
// 获取文件夹中的图像列表
const openFolderDialog = async () => {
    try {
        // 显示加载状态
        state.value.processingStatus = "正在打开文件选择器...";
        
        const selected = await window.__TAURI__.dialog.open({
            directory: true,
            multiple: true,
            title: "选择文件夹（按住 Ctrl/Command 键可选择多个）",
        });

        if (Array.isArray(selected)) {
            // 过滤已选择的文件夹
            const filteredFolders = selected.filter(
                folder => !newFolders.value.includes(folder)
            );
            
            if (filteredFolders.length > 0) {
                newFolders.value = [...newFolders.value, ...filteredFolders];
                state.value.processingStatus = `已添加 ${filteredFolders.length} 个文件夹`;
            } else {
                state.value.processingStatus = "未添加新文件夹";
            }
        } else if (selected && !newFolders.value.includes(selected)) {
            newFolders.value.push(selected);
            state.value.processingStatus = "已添加1个文件夹";
        } else if (selected === null) {
            state.value.processingStatus = "已取消选择";
        }
    } catch (err) {
        console.error("选择文件夹时出错：", err);
        state.value.processingStatus = `选择文件夹时出错: ${err}`;
    }
};

// 重新扫描
const rescan = async () => {
    if (newFolders.value.length === 0 || isRescanning.value) {
        return;
    }
        
    isRescanning.value = true;
    showSettings.value = false;
    state.value.processingStatus = "正在重新扫描...";
        
    try {
        // 准备请求参数
        const duplicateGroups = await window.__TAURI__.core.invoke("find_duplicates", {
            req: {
                folder_paths: newFolders.value,
                algorithm: newAlgorithm.value,
                similarity_threshold: Number(newThreshold.value),
                recursive: true, // 始终递归扫描子文件夹
            },
        });

        // 更新全局状态
        globalState.duplicateGroups = duplicateGroups || []; // 确保始终是数组，即使返回null或undefined
        globalState.selectedFolders = [...newFolders.value];
        globalState.algorithm = newAlgorithm.value;
        globalState.similarityThreshold = newThreshold.value;
        globalState.recursive = true; // 始终递归扫描子文件夹
        
        // 更新扫描统计信息
        if (!globalState.scanStats) {
            globalState.scanStats = {};
        }
        
        // 如果没有找到重复图片，确保统计信息正确
        if (!duplicateGroups || duplicateGroups.length === 0) {
            globalState.scanStats.processedImages = 0;
        }
            
        // 重新加载数据
        loadData();
    } catch (err) {
        console.error("重新扫描出错：", err);
        state.value.processingStatus = `重新扫描出错: ${err}`;
        // 清空当前重复组显示
        state.value.duplicateGroups = [];
        state.value.selectedImages = {};
    } finally {
        isRescanning.value = false;
    }
};

// 计算属性
const selectedCount = computed(() => {
    return Object.values(state.value.selectedImages).reduce((count, group) => {
        return count + Object.values(group).filter(Boolean).length;
    }, 0);
});

// 初始化组件
onMounted(() => {
    loadData();
    // 初始化滚动状态
    handleScroll();
});

// 滚动到指定分组
const scrollToGroup = (groupIndex) => {
    const element = document.getElementById(`group-${groupIndex}`);
    if (element) {
        element.scrollIntoView({ behavior: 'smooth', block: 'start' });
        // 滚动后立即隐藏导航菜单，不需要延迟
        groupNavVisible.value = false;
    }
};

// 监听路由参数变化
watch(
    () => route.params.timestamp,
    () => {
        loadData();
    },
);

// 监听滚动位置，显示/隐藏返回顶部按钮
onMounted(() => {
    window.addEventListener('scroll', handleScroll);
});

// 已在上面重新定义了事件监听，这里移除

// 分组导航显示状态
const groupNavVisible = ref(false);

// 是否在页面底部
const isAtBottom = ref(false);
// 鼠标是否悬停在底部导航按钮上
const isHoveringBottomNav = ref(false);

// 监听窗口大小变化和滚动事件，更新底部检测
onMounted(() => {
    window.addEventListener('resize', handleScroll);
    window.addEventListener('scroll', handleScroll);
});

onUnmounted(() => {
    window.removeEventListener('resize', handleScroll);
    window.removeEventListener('scroll', handleScroll);
});

// 处理滚动事件
const handleScroll = () => {
    // 当前滚动位置
    const scrollPosition = window.scrollY;
    const windowHeight = window.innerHeight;
    const scrollHeight = document.documentElement.scrollHeight;
    
    // 顶部按钮显示逻辑 - 滚动超过300px显示
    state.value.showBackToTop = scrollPosition > 300;
    
    // 分组导航显示逻辑 - 滚动超过300px且有足够分组时显示
    state.value.showGroupNav = scrollPosition > 300 && state.value.duplicateGroups.length > 3;
    
    // 检测是否滚动到底部 - 使用更严格的判断
    // 如果滚动位置 + 窗口高度接近文档总高度，认为到达底部
    // 添加50px的缓冲区以提高用户体验
    isAtBottom.value = scrollHeight - (scrollPosition + windowHeight) < 50;
    
    // 更新当前可见分组索引（用于导航菜单高亮）
    updateCurrentVisibleGroup();
    
    // 如果页面刚加载且没有滚动，但内容很短，需要默认隐藏底部按钮
    if (scrollPosition === 0 && scrollHeight <= windowHeight + 100) {
        isAtBottom.value = true;
    }
};

// 更新当前可见分组索引
const updateCurrentVisibleGroup = () => {
    if (state.value.duplicateGroups.length === 0) return;
    
    // 查找当前视口中最靠上的分组
    for (let i = 0; i < state.value.duplicateGroups.length; i++) {
        const element = document.getElementById(`group-${i}`);
        if (element) {
            const rect = element.getBoundingClientRect();
            // 如果元素的顶部在视口中，或者刚刚超出视口但底部仍可见
            if (rect.top <= 150 && rect.bottom > 0) {
                state.value.currentGroupIndex = i;
                return;
            }
        }
    }
};

// 滚动到顶部
const scrollToTop = () => {
    window.scrollTo({ top: 0, behavior: 'smooth' });
    
    // 在动画完成后检测位置以更新按钮状态
    setTimeout(() => {
        handleScroll();
    }, 1000);
};

// 显示分组导航
const showGroupNav = () => {
    groupNavVisible.value = true;
};

// 隐藏分组导航
const hideGroupNav = () => {
    // 立即隐藏导航菜单
    groupNavVisible.value = false;
};

// 滚动到底部
const scrollToBottom = () => {
    window.scrollTo({ 
        top: document.documentElement.scrollHeight, 
        behavior: 'smooth' 
    });
    
    // 在动画完成后检测位置以更新按钮状态
    setTimeout(() => {
        handleScroll();
    }, 1000);
};

// 加载数据
const loadData = async () => {
    try {
        // 清空当前状态，确保不会显示旧数据
        state.value.duplicateGroups = [];
        state.value.selectedImages = {};
        
        // 检查是否有重复组
        if (!globalState.duplicateGroups?.length) {
            state.value.processingStatus =
                "没有找到重复图片，或所有重复图片已处理完毕";
            
            // 确保统计信息正确
            if (globalState.scanStats) {
                state.value.scanStats = globalState.scanStats;
                state.value.scanStats.processedImages = 0;
            }
            
            return;
        }

        state.value.duplicateGroups = globalState.duplicateGroups
            .map(processGroup)
            .filter(Boolean);
        state.value.selectedFolders = globalState.selectedFolders || [];

        // 加载扫描统计信息
        if (globalState.scanStats) {
            state.value.scanStats = globalState.scanStats;
        } else {
            // 如果没有全局统计信息，尝试计算一些基本数据
            state.value.scanStats = {
                totalFolders: state.value.selectedFolders.length,
                totalFiles: 0, // 无法确定，显示为0
                totalImages: 0, // 将在后面更新
                processedImages: 0, // 将在后面更新
            };
        }

        // 初始化选择状态
        state.value.duplicateGroups.forEach((group, groupIndex) => {
            group.images.forEach((_, imageIndex) => {
                if (imageIndex > 0) {
                    if (!state.value.selectedImages[groupIndex]) {
                        state.value.selectedImages[groupIndex] = {};
                    }
                    state.value.selectedImages[groupIndex][imageIndex] = true;
                }
            });
        });

        const totalDuplicates = state.value.duplicateGroups.reduce(
            (sum, group) => sum + group.images.length,
            0,
        );
        
        // 更新处理的重复图像数
        state.value.scanStats.processedImages = totalDuplicates;
        
        // 如果globalState中没有totalImages，则计算唯一图像数
        if (!state.value.scanStats.totalImages) {
            // 计算所有唯一图像数量
            const uniqueImagePaths = new Set();
            state.value.duplicateGroups.forEach(group => {
                group.images.forEach(img => uniqueImagePaths.add(img.path));
            });
            state.value.scanStats.totalImages = uniqueImagePaths.size;
        }
        
        state.value.processingStatus = `处理完成，共找到 ${totalDuplicates} 张存在重复的图片，分为 ${state.value.duplicateGroups.length} 组`;
    } catch (err) {
        console.error("加载数据失败:", err);
        state.value.processingStatus = "加载数据失败，请重试";
        
        // 出错时清空数据
        state.value.duplicateGroups = [];
        state.value.selectedImages = {};
    }
};

// 格式化文件大小
const formatSize = (bytes) => {
    if (!bytes) return "0 B";
    const units = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return `${(bytes / Math.pow(1024, i)).toFixed(2)} ${units[i]}`;
};

// 打开图片
const openImage = async (path) => {
    try {
        await window.__TAURI__.opener.openPath(path);
    } catch (err) {
        console.error("打开图片失败:", err);
        state.value.processingStatus = `打开图片失败: ${err}`;
    }
};

// 预览图片
const previewImageFunc = (image, groupIndex, imageIndex) => {
    if (!image || groupIndex === undefined || imageIndex === undefined) return;

    state.value.previewImage = processImageMetadata(image);
    state.value.currentGroupIndex = groupIndex;
    state.value.currentImageIndex = imageIndex;
    state.value.showImagePreview = true;
};

// 切换到上一张图片
const prevImage = () => {
    try {
        const group =
            state.value.duplicateGroups[state.value.currentGroupIndex];
        if (!group || !group.images || group.images.length === 0) {
            console.error("无效的组数据");
            return;
        }

        let newIndex = state.value.currentImageIndex - 1;
        if (newIndex < 0) {
            newIndex = group.images.length - 1;
        }

        // 获取完整的图片对象
        const completeImage = group.images[newIndex];
        if (!completeImage) {
            console.error("无法获取图片数据:", newIndex);
            return;
        }

        // 深拷贝图片对象
        const imageCopy = JSON.parse(JSON.stringify(completeImage));

        // 更新状态
        state.value.currentImageIndex = newIndex;
        state.value.previewImage = imageCopy;

        console.log("切换到上一张图片:", {
            path: state.value.previewImage.path,
            size: state.value.previewImage.size_bytes,
            width: state.value.previewImage.width,
            height: state.value.previewImage.height,
            groupIndex: state.value.currentGroupIndex,
            imageIndex: newIndex,
            totalInGroup: group.images.length,
        });
    } catch (err) {
        console.error("切换图片时出错:", err);
    }
};

// 切换到下一张图片
const nextImage = () => {
    try {
        const group =
            state.value.duplicateGroups[state.value.currentGroupIndex];
        if (!group || !group.images || group.images.length === 0) {
            console.error("无效的组数据");
            return;
        }

        let newIndex = state.value.currentImageIndex + 1;
        if (newIndex >= group.images.length) {
            newIndex = 0;
        }

        // 获取完整的图片对象
        const completeImage = group.images[newIndex];
        if (!completeImage) {
            console.error("无法获取图片数据:", newIndex);
            return;
        }

        // 深拷贝图片对象
        const imageCopy = JSON.parse(JSON.stringify(completeImage));

        // 更新状态
        state.value.currentImageIndex = newIndex;
        state.value.previewImage = imageCopy;

        console.log("切换到下一张图片:", {
            path: state.value.previewImage.path,
            size: state.value.previewImage.size_bytes,
            width: state.value.previewImage.width,
            height: state.value.previewImage.height,
            groupIndex: state.value.currentGroupIndex,
            imageIndex: newIndex,
            totalInGroup: group.images.length,
        });
    } catch (err) {
        console.error("切换图片时出错:", err);
    }
};

// 关闭预览
const closePreview = () => {
    state.value.showImagePreview = false;
    state.value.previewImage = null;
};

// 切换当前预览图片的选择状态
const toggleCurrentImageSelection = () => {
    toggleImageSelection(
        state.value.currentGroupIndex,
        state.value.currentImageIndex,
    );
};

// 删除当前预览的图片
const deleteCurrentImage = async () => {
    if (
        !(await window.__TAURI__.dialog.confirm(
            "确定要删除当前图片吗？此操作不可恢复！",
        ))
    ) {
        return;
    }

    const path = state.value.previewImage.path;

    try {
        // 调用 Tauri 删除文件
        await window.__TAURI__.fs.remove(path);

        // 更新界面，移除已删除的图片
        const group =
            state.value.duplicateGroups[state.value.currentGroupIndex];
        group.images = group.images.filter((image) => image.path !== path);

        // 如果组中只剩一张图片，关闭预览并过滤掉该组
        if (group.images.length <= 1) {
            state.value.duplicateGroups = state.value.duplicateGroups.filter(
                (_, index) => index !== state.value.currentGroupIndex,
            );
            closePreview();
            state.value.processingStatus =
                "已成功删除图片，该组中的重复图片已全部处理";
            return;
        }

        // 调整当前索引
        if (state.value.currentImageIndex >= group.images.length) {
            state.value.currentImageIndex = group.images.length - 1;
        }

        // 更新预览图片
        state.value.previewImage = group.images[state.value.currentImageIndex];

        state.value.processingStatus = "已成功删除图片";
    } catch (err) {
        console.error("删除图片时出错:", err);
        state.value.processingStatus = `删除图片时出错: ${err}`;
    }
};

// 切换图片选择状态
const toggleImageSelection = (groupIndex, imageIndex) => {
    if (!state.value.selectedImages[groupIndex]) {
        state.value.selectedImages[groupIndex] = {};
    }

    state.value.selectedImages[groupIndex][imageIndex] =
        !state.value.selectedImages[groupIndex][imageIndex];
};

// 检查图片是否被选中
const isImageSelected = (groupIndex, imageIndex) => {
    return state.value.selectedImages[groupIndex]?.[imageIndex] || false;
};

// 获取所有选中图片的路径
const getSelectedImagePaths = () => {
    const paths = [];

    Object.entries(state.value.selectedImages).forEach(
        ([groupIndex, group]) => {
            Object.entries(group).forEach(([imageIndex, selected]) => {
                if (selected) {
                    paths.push(
                        state.value.duplicateGroups[parseInt(groupIndex)]
                            .images[parseInt(imageIndex)].path,
                    );
                }
            });
        },
    );

    return paths;
};

// 删除选中的图片
const deleteSelectedImages = async () => {
    if (selectedCount.value === 0) {
        state.value.processingStatus = "请先选择要删除的图片";
        return;
    }

    // 获取选中图片的路径，用于提示用户
    const imagePaths = getSelectedImagePaths();

    if (
        !(await window.__TAURI__.dialog.confirm(
            `确定要删除选中的 ${imagePaths.length} 张图片吗？此操作不可恢复！`,
        ))
    ) {
        return;
    }

    // 提示用户正在删除图片
    state.value.processingStatus = `正在删除 ${imagePaths.length} 张图片...`;

    try {
        // 调用 Tauri 删除文件
        for (const path of imagePaths) {
            await window.__TAURI__.fs.remove(path);
        }

        state.value.processingStatus = `已成功删除 ${imagePaths.length} 张图片`;

        // 更新界面，移除已删除的图片
        state.value.duplicateGroups = state.value.duplicateGroups
            .map((group) => ({
                ...group,
                images: group.images.filter(
                    (image) => !imagePaths.includes(image.path),
                ),
            }))
            .filter((group) => group.images.length > 1); // 只保留仍有多张图片的组

        // 重置选择状态
        state.value.selectedImages = {};
    } catch (err) {
        console.error("删除图片时出错:", err);
        state.value.processingStatus = `删除图片时出错: ${err}`;
    }
};

// 取消选择所有图片 -> 全选/全不选所有图片
const deselectAllImages = () => {
    // 检查当前是否有选中的图片
    const hasSelected = Object.values(state.value.selectedImages).some(
        (group) => Object.values(group).some((selected) => selected),
    );

    if (hasSelected) {
        // 如果有选中的图片，则全部取消选择
        state.value.selectedImages = {};
        state.value.processingStatus = "已取消所有选择";
    } else {
        // 如果没有选中的图片，则全部选择
        state.value.duplicateGroups.forEach((group, groupIndex) => {
            group.images.forEach((_, imageIndex) => {
                if (!state.value.selectedImages[groupIndex]) {
                    state.value.selectedImages[groupIndex] = {};
                }
                state.value.selectedImages[groupIndex][imageIndex] = true;
            });
        });
        state.value.processingStatus = "已选择所有图片";
    }
};

// 返回首页
const goBack = () => {
    router.push({ name: "home" });
};

// 全选/取消全选当前组
const toggleGroupSelection = (groupIndex) => {
    const group = state.value.duplicateGroups[groupIndex];
    const allSelected = group.images.every((_, imageIndex) =>
        isImageSelected(groupIndex, imageIndex),
    );

    // 所有图片
    for (let i = 0; i < group.images.length; i++) {
        if (!state.value.selectedImages[groupIndex]) {
            state.value.selectedImages[groupIndex] = {};
        }
        state.value.selectedImages[groupIndex][i] = !allSelected;
    }
};

// 获取图片的相对路径（用于显示）
const getRelativePath = (path) => {
    if (!path) return "";
    for (const folder of state.value.selectedFolders) {
        if (path.startsWith(folder)) {
            return path.substring(folder.length + 1);
        }
    }
    return path;
};

// 将图片转换为 Data URL 以在前端显示
const getImageDataUrl = (path) => {
    // 使用Tauri的convertFileSrc函数将文件路径转换为可访问的URL
    return window.__TAURI__.core.convertFileSrc(path);
};

// 智能选择功能
const smartSelect = (criteria) => {
    state.value.duplicateGroups.forEach((group, groupIndex) => {
        if (!state.value.selectedImages[groupIndex]) {
            state.value.selectedImages[groupIndex] = {};
        }

        // 根据不同的标准选择图片
        let bestImageIndex = 0;
        switch (criteria) {
            case "quality":
                // 质量最佳：综合考虑分辨率和文件大小
                bestImageIndex = group.images.reduce((best, current, index) => {
                    const bestScore =
                        (group.images[best].width * group.images[best].height) /
                        group.images[best].size_bytes;
                    const currentScore =
                        (current.width * current.height) / current.size_bytes;
                    return currentScore > bestScore ? index : best;
                }, 0);
                break;

            case "size":
                // 文件最大
                bestImageIndex = group.images.reduce((best, current, index) => {
                    return current.size_bytes > group.images[best].size_bytes
                        ? index
                        : best;
                }, 0);
                break;

            case "resolution":
                // 分辨率最大
                bestImageIndex = group.images.reduce((best, current, index) => {
                    const bestPixels =
                        group.images[best].width * group.images[best].height;
                    const currentPixels = current.width * current.height;
                    return currentPixels > bestPixels ? index : best;
                }, 0);
                break;

            case "oldest":
                // 时间最早
                bestImageIndex = group.images.reduce((best, current, index) => {
                    const currentDate = current.created_at
                        ? new Date(current.created_at)
                        : new Date(0);
                    const bestDate = group.images[best].created_at
                        ? new Date(group.images[best].created_at)
                        : new Date(0);
                    return currentDate < bestDate ? index : best;
                }, 0);
                break;

            case "newest":
                // 时间最新
                bestImageIndex = group.images.reduce((best, current, index) => {
                    const currentDate = current.modified_at
                        ? new Date(current.modified_at)
                        : current.created_at
                          ? new Date(current.created_at)
                          : new Date(0);
                    const bestDate = group.images[best].modified_at
                        ? new Date(group.images[best].modified_at)
                        : group.images[best].created_at
                          ? new Date(group.images[best].created_at)
                          : new Date(0);
                    return currentDate > bestDate ? index : best;
                }, 0);
                break;
        }

        // 选择除最佳图片外的所有图片
        group.images.forEach((_, imageIndex) => {
            state.value.selectedImages[groupIndex][imageIndex] =
                imageIndex !== bestImageIndex;
        });
    });

    state.value.processingStatus = `已根据${getCriteriaName(criteria)}智能选择图片`;
};

// 获取标准名称
const getCriteriaName = (criteria) => {
    const names = {
        quality: "质量最佳",
        size: "文件最大",
        resolution: "分辨率最大",
        oldest: "创建时间",
        newest: "修改时间",
    };
    return names[criteria] || criteria;
};

// 格式化日期
const formatDate = (timestamp) => {
    if (!timestamp || timestamp === "0") return "未知时间";

    // 处理 Unix 时间戳（秒）
    const timestampNum = parseInt(timestamp);
    if (isNaN(timestampNum)) return "未知时间";

    // 将秒转换为毫秒
    const date = new Date(timestampNum * 1000);
    if (isNaN(date.getTime())) return "未知时间";

    const now = new Date();
    const diff = now - date;
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));

    if (days === 0) {
        // 今天
        return `今天 ${date.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" })}`;
    } else if (days === 1) {
        // 昨天
        return `昨天 ${date.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" })}`;
    } else if (days < 7) {
        // 一周内
        return `${days}天前`;
    } else {
        // 更早
        return date.toLocaleDateString("zh-CN", {
            year: "numeric",
            month: "2-digit",
            day: "2-digit",
            hour: "2-digit",
            minute: "2-digit",
        });
    }
};

// 处理图片元数据
const processImageMetadata = (image) => {
    return {
        ...image,
        formattedSize: formatSize(image.size_bytes),
        resolution: `${image.width} × ${image.height}`,
        createdDate: formatDate(image.created_at),
        modifiedDate: formatDate(image.modified_at || image.created_at),
        formattedDate: formatDate(image.modified_at || image.created_at),
        relativePath: getRelativePath(image.path),
        dataUrl: window.__TAURI__.core.convertFileSrc(image.path),
    };
};

// 图片组处理
const processGroup = (group) => {
    if (!group || !group.images) return null;

    return {
        ...group,
        images: group.images.map(processImageMetadata).filter(Boolean),
        totalSize: group.images.reduce(
            (sum, img) => sum + (img.size_bytes || 0),
            0,
        ),
        averageQuality:
            group.images.reduce(
                (sum, img) => sum + (img.width * img.height) / img.size_bytes,
                0,
            ) / group.images.length,
    };
};
</script>

<style>
@keyframes fadeIn {
    from {
        opacity: 0;
        transform: translateY(10px);
    }
    to {
        opacity: 1;
        transform: translateY(0);
    }
}
@keyframes slideIn {
    from {
        opacity: 0;
        transform: translateX(-20px);
    }
    to {
        opacity: 1;
        transform: translateX(0);
    }
}
@keyframes slideOut {
    from {
        opacity: 1;
        transform: translateX(0);
    }
    to {
        opacity: 0;
        transform: translateX(-20px);
    }
}
@keyframes pulse {
    0% {
        box-shadow: 0 0 0 0 rgba(59, 130, 246, 0.4);
    }
    70% {
        box-shadow: 0 0 0 6px rgba(59, 130, 246, 0);
    }
    100% {
        box-shadow: 0 0 0 0 rgba(59, 130, 246, 0);
    }
}
.animate-fade-in {
    animation: fadeIn 0.5s ease forwards;
}
.animate-slide-in {
    animation: slideIn 0.3s cubic-bezier(0.34, 1.56, 0.64, 1) forwards;
}
.animate-slide-out {
    animation: slideOut 0.3s cubic-bezier(0.34, 1.56, 0.64, 1) forwards;
}
.animate-pulse {
    animation: pulse 2s infinite;
}
.fade-enter-active, .fade-leave-active {
    transition: opacity 0.3s, transform 0.3s;
}
.fade-enter-from, .fade-leave-to {
    opacity: 0;
    transform: translateY(10px);
}
.scrollbar-thin {
    scrollbar-width: thin;
}
.scrollbar-thin::-webkit-scrollbar {
    width: 4px;
}
.scrollbar-thin::-webkit-scrollbar-track {
    background: #f1f1f1;
    border-radius: 4px;
}
.scrollbar-thin::-webkit-scrollbar-thumb {
    background: #888;
    border-radius: 4px;
}
.scrollbar-thin::-webkit-scrollbar-thumb:hover {
    background: #555;
}
</style>

<template>
    <main class="p-8 max-w-7xl mx-auto relative">
        <!-- 调整参数快捷菜单 -->
        <div class="fixed right-6 top-6 z-40">
            <button 
                class="bg-white p-3 rounded-full shadow-lg hover:bg-slate-50 hover:text-blue-600 transform hover:scale-105 active:scale-95 transition-all duration-300 text-slate-600"
                title="调整扫描参数"
                @click="showSettings = !showSettings"
            >
                <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"></path>
                    <circle cx="12" cy="12" r="3"></circle>
                </svg>
            </button>
            
            <!-- 设置面板 -->
            <transition
                enter-active-class="transition ease-out duration-300 transform"
                enter-from-class="opacity-0 scale-95 translate-y-2"
                enter-to-class="opacity-100 scale-100 translate-y-0"
                leave-active-class="transition ease-in duration-200 transform"
                leave-from-class="opacity-100 scale-100 translate-y-0"
                leave-to-class="opacity-0 scale-95 translate-y-2"
            >
                <div v-if="showSettings" class="absolute top-14 right-0 w-80 bg-white rounded-xl shadow-xl border border-slate-200 overflow-hidden">
                    <div class="p-4 bg-slate-50 border-b border-slate-200">
                        <h3 class="text-lg font-semibold text-slate-800">调整扫描参数</h3>
                        <p class="text-sm text-slate-600">修改后点击"重新扫描"应用更改</p>
                    </div>
                    <div class="p-4">
                        <!-- 算法选择 -->
                        <div class="mb-4">
                            <label class="block text-sm font-medium text-slate-700 mb-2 flex items-center gap-2">
                                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-slate-500">
                                    <rect width="18" height="18" x="3" y="3" rx="2" ry="2"></rect>
                                    <path d="M7 17v-4"></path>
                                    <path d="M11 17v-8"></path>
                                    <path d="M15 17v-4"></path>
                                    <path d="M19 17v-8"></path>
                                </svg>
                                检测算法
                            </label>
                            <div class="grid grid-cols-1 sm:grid-cols-3 gap-2 mb-1">
                                <button
                                    v-for="algo in algorithms"
                                    :key="algo.id"
                                    type="button"
                                    @click="newAlgorithm = algo.id"
                                    :class="{
                                        'px-3 py-2 rounded-lg text-sm transition-all duration-300 border flex items-center justify-center gap-1': true,
                                        'bg-blue-50 border-blue-200 text-blue-600 shadow-sm': newAlgorithm === algo.id,
                                        'bg-white border-slate-200 text-slate-700 hover:bg-slate-50': newAlgorithm !== algo.id
                                    }"
                                >
                                    <svg v-if="newAlgorithm === algo.id" xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                        <polyline points="20 6 9 17 4 12"></polyline>
                                    </svg>
                                    {{ algo.name }}
                                </button>
                            </div>
                        </div>
                        
                        <!-- 相似度阈值 -->
                        <div class="mb-4" v-if="newAlgorithm !== 'Exact'">
                            <label class="block text-sm font-medium text-slate-700 mb-2">
                                相似度阈值: {{ newThreshold }}%
                            </label>
                            <input 
                                type="range" 
                                v-model="newThreshold" 
                                min="0" 
                                max="100" 
                                step="1"
                                class="w-full h-2 bg-slate-200 rounded-lg appearance-none cursor-pointer"
                            />
                        </div>
                        
                        <!-- 文件夹选择 -->
                        <div class="mb-4">
                            <label class="block text-sm font-medium text-slate-700 mb-2 flex items-center gap-2">
                                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-slate-500">
                                    <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"></path>
                                </svg>
                                扫描文件夹
                            </label>
                            <div class="overflow-hidden rounded-lg border border-slate-200 shadow-sm mb-2">
                                <div class="max-h-32 overflow-y-auto scrollbar-thin divide-y divide-slate-100">
                                    <div v-if="newFolders.length === 0" class="flex items-center justify-center p-3 text-sm text-slate-500 bg-slate-50 italic">
                                        未选择文件夹
                                    </div>
                                    <div 
                                        v-for="(folder, index) in newFolders" 
                                        :key="index"
                                        class="flex items-center justify-between p-2.5 bg-white hover:bg-slate-50 text-sm group transition-colors"
                                    >
                                        <div class="flex items-center gap-2 truncate text-slate-700">
                                            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-amber-500 flex-shrink-0">
                                                <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"></path>
                                            </svg>
                                            <span class="truncate">{{ folder }}</span>
                                        </div>
                                        <button 
                                            @click="newFolders.splice(index, 1)"
                                            class="p-1 text-slate-400 hover:text-red-500 rounded-full opacity-0 group-hover:opacity-100 transition-opacity"
                                            title="移除此文件夹"
                                        >
                                            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                                <line x1="18" y1="6" x2="6" y2="18"></line>
                                                <line x1="6" y1="6" x2="18" y2="18"></line>
                                            </svg>
                                        </button>
                                    </div>
                                </div>
                                <div class="bg-slate-50 p-2 border-t border-slate-100">
                                    <button 
                                        @click="openFolderDialog"
                                        class="w-full py-2 bg-white border border-slate-200 text-slate-700 rounded-lg hover:bg-slate-50 hover:border-slate-300 transition-all flex items-center justify-center gap-2 shadow-sm transform hover:-translate-y-0.5 active:translate-y-0 duration-300"
                                    >
                                        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-blue-500">
                                            <path d="M5 4h4l3 3h7a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2z"></path>
                                            <line x1="12" y1="11" x2="12" y2="17"></line>
                                            <line x1="9" y1="14" x2="15" y2="14"></line>
                                        </svg>
                                        添加文件夹
                                    </button>
                                </div>
                            </div>
                        </div>
                        
                        <!-- 移除递归选项，默认递归扫描 -->
                        
                        <!-- 操作按钮 -->
                        <div class="flex justify-between mt-6">
                            <button 
                                @click="showSettings = false"
                                class="px-4 py-2 bg-slate-100 text-slate-700 rounded-lg hover:bg-slate-200 transform hover:-translate-y-0.5 active:translate-y-0 transition-all duration-300 shadow-sm"
                            >
                                取消
                            </button>
                            <button 
                                @click="rescan"
                                class="px-4 py-2 bg-blue-500 text-white rounded-lg hover:bg-blue-600 transform hover:-translate-y-0.5 active:translate-y-0 transition-all duration-300 shadow-sm disabled:bg-blue-300 disabled:cursor-not-allowed disabled:transform-none flex items-center gap-2"
                                :disabled="newFolders.length === 0 || isRescanning"
                            >
                                <svg v-if="isRescanning" xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="animate-spin">
                                    <path d="M21 12a9 9 0 1 1-6.219-8.56"></path>
                                </svg>
                                <span v-if="isRescanning">扫描中...</span>
                                <span v-else>重新扫描</span>
                            </button>
                        </div>
                    </div>
                </div>
            </transition>
        </div>
        <div class="w-full">
            <div class="flex items-center mb-8 gap-4">
                <button
                    class="flex items-center gap-2 px-4 py-2 bg-slate-100 rounded-lg font-medium transition-all hover:bg-slate-200 transform hover:-translate-y-0.5 active:translate-y-0 duration-300"
                    @click="goBack"
                >
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        width="20"
                        height="20"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <line x1="19" y1="12" x2="5" y2="12"></line>
                        <polyline points="12 19 5 12 12 5"></polyline>
                    </svg>
                    返回首页
                </button>
                <h1 class="text-2xl font-bold text-slate-900">重复图片处理</h1>
            </div>

            <!-- 没有找到重复图片时的提示界面 -->
            <div
                v-if="!globalState.duplicateGroups?.length"
                class="flex flex-col items-center justify-center p-16 text-center bg-white rounded-xl shadow-sm"
            >
                <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="64"
                    height="64"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    class="text-slate-400 mb-6"
                >
                    <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
                    <circle cx="8.5" cy="8.5" r="1.5" />
                    <polyline points="21 15 16 10 5 21" />
                </svg>
                <p class="text-slate-500 text-lg mb-8">
                    {{ state.processingStatus }}
                </p>
                <button
                    class="mx-auto mb-4 flex items-center gap-2 px-6 py-3 bg-blue-600 text-white rounded-xl font-semibold shadow-lg hover:bg-blue-700 transition-all transform hover:-translate-y-1 active:translate-y-0"
                    @click="goBack"
                >
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        width="20"
                        height="20"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <path d="M19 12H5M12 19l-7-7 7-7" />
                    </svg>
                    返回首页重新检测
                </button>
            </div>

            <div v-else class="flex flex-col gap-4 mb-6">
                <div
                    v-if="state.processingStatus"
                    class="p-4 bg-slate-50 rounded-lg border border-slate-200"
                >
                    <div class="flex flex-col gap-2">
                        <p class="text-sm text-slate-600">
                            {{ state.processingStatus }}
                        </p>
                        <!-- 扫描统计信息 -->
                        <div class="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-6 gap-4 mt-2">
                            <div class="bg-white p-3 rounded-lg shadow-sm flex items-center gap-3">
                                <div class="bg-blue-100 p-2 rounded-lg">
                                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-blue-600">
                                        <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"></path>
                                    </svg>
                                </div>
                                <div>
                                    <p class="text-xs text-slate-500">文件夹数</p>
                                    <p class="text-lg font-semibold">{{ state.scanStats.totalFolders || state.selectedFolders.length }}</p>
                                </div>
                            </div>
                            <div class="bg-white p-3 rounded-lg shadow-sm flex items-center gap-3">
                                <div class="bg-green-100 p-2 rounded-lg">
                                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-green-600">
                                        <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"></path>
                                        <polyline points="14 2 14 8 20 8"></polyline>
                                    </svg>
                                </div>
                                <div>
                                    <p class="text-xs text-slate-500">总文件数</p>
                                    <p class="text-lg font-semibold">{{ state.scanStats.totalFiles || '-' }}</p>
                                </div>
                            </div>
                            <div class="bg-white p-3 rounded-lg shadow-sm flex items-center gap-3">
                                <div class="bg-purple-100 p-2 rounded-lg">
                                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-purple-600">
                                        <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
                                        <circle cx="8.5" cy="8.5" r="1.5"></circle>
                                        <polyline points="21 15 16 10 5 21"></polyline>
                                    </svg>
                                </div>
                                <div>
                                    <p class="text-xs text-slate-500">图像总数</p>
                                    <p class="text-lg font-semibold">{{ state.scanStats.totalImages }}</p>
                                </div>
                            </div>
                            <div class="bg-white p-3 rounded-lg shadow-sm flex items-center gap-3">
                                <div class="bg-amber-100 p-2 rounded-lg">
                                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-amber-600">
                                        <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"></path>
                                        <circle cx="9" cy="7" r="4"></circle>
                                        <path d="M23 21v-2a4 4 0 0 0-3-3.87"></path>
                                        <path d="M16 3.13a4 4 0 0 1 0 7.75"></path>
                                    </svg>
                                </div>
                                <div>
                                    <p class="text-xs text-slate-500">重复图像</p>
                                    <p class="text-lg font-semibold">{{ state.scanStats.processedImages }}</p>
                                </div>
                            </div>
                            <div class="bg-white p-3 rounded-lg shadow-sm flex items-center gap-3">
                                <div class="bg-indigo-100 p-2 rounded-lg">
                                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-indigo-600">
                                        <path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"></path>
                                        <circle cx="12" cy="12" r="3"></circle>
                                    </svg>
                                </div>
                                <div>
                                    <p class="text-xs text-slate-500">使用算法</p>
                                    <p class="text-lg font-semibold">{{ globalState.algorithm }}</p>
                                </div>
                            </div>
                            <div class="bg-white p-3 rounded-lg shadow-sm flex items-center gap-3">
                                <div class="bg-teal-100 p-2 rounded-lg">
                                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-teal-600">
                                        <path d="M2 12h10"></path>
                                        <path d="M9 4v16"></path>
                                        <path d="M14 9h2a2 2 0 0 1 2 2v2a2 2 0 0 1-2 2h-2"></path>
                                        <path d="M22 16a6 6 0 0 1-6 6"></path>
                                    </svg>
                                </div>
                                <div>
                                    <p class="text-xs text-slate-500">{{ globalState.algorithm === 'Exact' ? '重复组数' : '相似度阈值' }}</p>
                                    <p class="text-lg font-semibold">{{ globalState.algorithm === 'Exact' ? state.duplicateGroups.length : globalState.similarityThreshold + '%' }}</p>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>

                <!-- 工具栏 -->
                <div
                    v-if="state.duplicateGroups.length > 0"
                    class="flex flex-wrap items-center justify-between gap-4 p-4 bg-white rounded-lg shadow-sm border border-slate-200"
                >
                    <!-- 智能选择 -->
                    <div class="flex items-center gap-2">
                        <div
                            class="flex items-center gap-2 text-sm text-slate-600"
                        >
                            <svg
                                xmlns="http://www.w3.org/2000/svg"
                                width="16"
                                height="16"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <path
                                    d="M12 2a10 10 0 1 0 10 10 4 4 0 0 1-5-5 4 4 0 0 1-5-5"
                                />
                                <path d="M8.5 8.5v.01" />
                                <path d="M16 15.5v.01" />
                                <path d="M12 12v.01" />
                                <path d="M11 17v.01" />
                                <path d="M7 14v.01" />
                            </svg>
                            智能选择：
                        </div>
                        <div class="flex flex-wrap gap-2">
                            <button
                                v-for="criteria in [
                                    'quality',
                                    'size',
                                    'resolution',
                                    'oldest',
                                    'newest',
                                ]"
                                :key="criteria"
                                @click="smartSelect(criteria)"
                                class="px-3 py-1.5 text-sm bg-slate-100 hover:bg-slate-200 text-slate-700 rounded-lg transition-all duration-300 transform hover:scale-105 active:scale-95"
                            >
                                {{ getCriteriaName(criteria) }}
                            </button>
                        </div>
                    </div>

                    <!-- 操作按钮 -->
                    <div class="flex items-center gap-2">
                        <button
                            class="flex items-center gap-2 px-4 py-2 bg-slate-100 rounded-lg font-medium transition-all hover:bg-slate-200 transform hover:-translate-y-0.5 active:translate-y-0 duration-300"
                            @click="deselectAllImages"
                        >
                            <svg
                                xmlns="http://www.w3.org/2000/svg"
                                width="16"
                                height="16"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <rect
                                    x="3"
                                    y="3"
                                    width="18"
                                    height="18"
                                    rx="2"
                                    ry="2"
                                />
                                <line x1="9" y1="9" x2="15" y2="15" />
                            </svg>
                            {{ selectedCount === 0 ? "全选" : "取消选择" }}
                        </button>
                        <button
                            class="flex items-center gap-2 px-4 py-2 bg-red-500 text-white rounded-lg font-medium transition-all hover:bg-red-600 disabled:bg-red-300 disabled:cursor-not-allowed transform hover:-translate-y-0.5 active:translate-y-0 duration-300"
                            @click="deleteSelectedImages"
                            :disabled="selectedCount === 0"
                        >
                            <svg
                                xmlns="http://www.w3.org/2000/svg"
                                width="16"
                                height="16"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <path d="M3 6h18" />
                                <path
                                    d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"
                                />
                                <path
                                    d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
                                />
                            </svg>
                            删除选中的图片 ({{ selectedCount }})
                        </button>
                    </div>
                </div>
            </div>

            <!-- 分组导航菜单 -->
            <!-- 移除顶部快速跳转分组组件，统一使用左侧悬浮导航 -->
    
            <!-- 显示重复图片组 -->
                    <div
                        v-if="state.duplicateGroups.length > 0"
                        class="mt-4 animate-fade-in relative"
                    >
                        <div class="flex flex-col gap-8">
                    <div
                        v-for="(group, groupIndex) in state.duplicateGroups"
                        :key="groupIndex"
                        :id="`group-${groupIndex}`"
                        class="bg-white rounded-xl shadow-sm overflow-hidden border border-slate-200 transition-all hover:shadow-md"
                    >
                        <div
                            class="flex justify-between items-center p-4 bg-slate-50 border-b border-slate-200"
                        >
                            <h4 class="text-lg text-gray-900">
                                重复组 #{{ groupIndex + 1 }}
                                <span
                                    class="text-slate-500 font-normal text-sm ml-2"
                                    >({{ group.images.length }}张图片)</span
                                >
                            </h4>
                            <div class="flex items-center gap-2">
                                <button
                                    class="flex items-center gap-2 px-3 py-2 bg-slate-100 rounded-lg text-sm transition-all hover:bg-slate-200"
                                    @click="toggleGroupSelection(groupIndex)"
                                >
                                    <svg
                                        xmlns="http://www.w3.org/2000/svg"
                                        width="16"
                                        height="16"
                                        viewBox="0 0 24 24"
                                        fill="none"
                                        stroke="currentColor"
                                        stroke-width="2"
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                    >
                                        <polyline
                                            points="9 11 12 14 22 4"
                                        ></polyline>
                                        <path
                                            d="M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11"
                                        ></path>
                                    </svg>
                                    全选/取消
                                </button>
                            </div>
                        </div>

                        <div
                            class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4 p-6"
                        >
                            <div
                                v-for="(image, imageIndex) in group.images"
                                :key="imageIndex"
                                class="border border-slate-200 rounded-lg overflow-hidden transition-all hover:-translate-y-1 hover:shadow-md"
                            >
                                <div
                                    class="h-48 overflow-hidden cursor-pointer relative group"
                                    @click="
                                        previewImageFunc(
                                            image,
                                            groupIndex,
                                            imageIndex,
                                        )
                                    "
                                >
                                    <img
                                        :src="image.dataUrl"
                                        alt="图片预览"
                                        loading="lazy"
                                        class="w-full h-full object-cover transition-transform group-hover:scale-105"
                                    />
                                    <div
                                        class="absolute inset-0 bg-black/40 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center"
                                    >
                                        <svg
                                            xmlns="http://www.w3.org/2000/svg"
                                            width="32"
                                            height="32"
                                            viewBox="0 0 24 24"
                                            fill="none"
                                            stroke="white"
                                            stroke-width="2"
                                            stroke-linecap="round"
                                            stroke-linejoin="round"
                                        >
                                            <circle cx="11" cy="11" r="8" />
                                            <line
                                                x1="21"
                                                y1="21"
                                                x2="16.65"
                                                y2="16.65"
                                            />
                                        </svg>
                                    </div>
                                </div>
                                <div class="p-4 flex items-start gap-3">
                                    <div class="pt-1">
                                        <input
                                            type="checkbox"
                                            :id="`img-${groupIndex}-${imageIndex}`"
                                            :checked="
                                                isImageSelected(
                                                    groupIndex,
                                                    imageIndex,
                                                )
                                            "
                                            @change="
                                                toggleImageSelection(
                                                    groupIndex,
                                                    imageIndex,
                                                )
                                            "
                                            class="rounded border-slate-300 text-blue-600 focus:ring-blue-500"
                                        />
                                    </div>
                                    <div class="flex-1 overflow-hidden">
                                        <div
                                            class="text-sm text-slate-700 mb-2 truncate"
                                            :title="image.path"
                                        >
                                            {{ image.relativePath }}
                                        </div>
                                        <div
                                            class="flex flex-wrap gap-4 text-xs text-slate-500"
                                        >
                                            <span
                                                class="flex items-center gap-1"
                                            >
                                                <svg
                                                    xmlns="http://www.w3.org/2000/svg"
                                                    width="12"
                                                    height="12"
                                                    viewBox="0 0 24 24"
                                                    fill="none"
                                                    stroke="currentColor"
                                                    stroke-width="2"
                                                    stroke-linecap="round"
                                                    stroke-linejoin="round"
                                                >
                                                    <path
                                                        d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"
                                                    />
                                                    <polyline
                                                        points="7 10 12 15 17 10"
                                                    />
                                                    <line
                                                        x1="12"
                                                        y1="15"
                                                        x2="12"
                                                        y2="3"
                                                    />
                                                </svg>
                                                {{ image.formattedSize }}
                                            </span>
                                            <span
                                                class="flex items-center gap-1"
                                            >
                                                <svg
                                                    xmlns="http://www.w3.org/2000/svg"
                                                    width="12"
                                                    height="12"
                                                    viewBox="0 0 24 24"
                                                    fill="none"
                                                    stroke="currentColor"
                                                    stroke-width="2"
                                                    stroke-linecap="round"
                                                    stroke-linejoin="round"
                                                >
                                                    <rect
                                                        x="3"
                                                        y="3"
                                                        width="18"
                                                        height="18"
                                                        rx="2"
                                                        ry="2"
                                                    />
                                                    <circle
                                                        cx="8.5"
                                                        cy="8.5"
                                                        r="1.5"
                                                    />
                                                    <polyline
                                                        points="21 15 16 10 5 21"
                                                    />
                                                </svg>
                                                {{ image.resolution }}
                                            </span>
                                            <span
                                                class="flex items-center gap-1"
                                            >
                                                <svg
                                                    xmlns="http://www.w3.org/2000/svg"
                                                    width="12"
                                                    height="12"
                                                    viewBox="0 0 24 24"
                                                    fill="none"
                                                    stroke="currentColor"
                                                    stroke-width="2"
                                                    stroke-linecap="round"
                                                    stroke-linejoin="round"
                                                >
                                                    <circle
                                                        cx="12"
                                                        cy="12"
                                                        r="10"
                                                    />
                                                    <polyline
                                                        points="12 6 12 12 16 14"
                                                    />
                                                </svg>
                                                {{ image.modifiedDate }}
                                            </span>
                                        </div>
                                    </div>
                                    <div class="flex gap-2">
                                        <button
                                            class="p-1.5 text-slate-500 rounded hover:bg-slate-100 hover:text-blue-600 transition-colors"
                                            @click="openImage(image.path)"
                                            title="打开图片"
                                        >
                                            <svg
                                                xmlns="http://www.w3.org/2000/svg"
                                                width="16"
                                                height="16"
                                                viewBox="0 0 24 24"
                                                fill="none"
                                                stroke="currentColor"
                                                stroke-width="2"
                                                stroke-linecap="round"
                                                stroke-linejoin="round"
                                            >
                                                <path
                                                    d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"
                                                />
                                                <polyline
                                                    points="15 3 21 3 21 9"
                                                />
                                                <line
                                                    x1="10"
                                                    y1="14"
                                                    x2="21"
                                                    y2="3"
                                                />
                                            </svg>
                                        </button>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>

        <!-- 图片预览模态框 -->
        <div
            v-if="state.showImagePreview && state.previewImage"
            class="fixed inset-0 bg-black/75 flex items-center justify-center z-50 p-8"
            @click="closePreview"
        >
            <div
                class="bg-white rounded-xl w-[90%] max-w-[1000px] max-h-[90vh] overflow-hidden flex flex-col"
                @click.stop
            >
                <div
                    class="flex justify-between items-center p-4 border-b border-slate-200"
                >
                    <h3 class="text-xl font-semibold">图片预览</h3>
                    <button
                        class="p-2 text-slate-500 rounded-full hover:bg-slate-100 hover:text-red-500 transition-colors"
                        @click="closePreview"
                    >
                        <svg
                            xmlns="http://www.w3.org/2000/svg"
                            width="24"
                            height="24"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        >
                            <line x1="18" y1="6" x2="6" y2="18"></line>
                            <line x1="6" y1="6" x2="18" y2="18"></line>
                        </svg>
                    </button>
                </div>
                <div
                    class="p-6 overflow-y-auto flex-1 flex flex-col items-center"
                >
                    <div class="flex items-center w-full relative mb-6">
                        <button
                            class="absolute top-1/2 -translate-y-1/2 left-2.5 bg-white/80 border-none rounded-full w-10 h-10 flex items-center justify-center cursor-pointer z-10 shadow-md hover:bg-white hover:shadow-lg transition-all"
                            @click.stop="prevImage"
                        >
                            <svg
                                xmlns="http://www.w3.org/2000/svg"
                                width="24"
                                height="24"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <polyline points="15 18 9 12 15 6"></polyline>
                            </svg>
                        </button>
                        <div class="w-full flex justify-center">
                            <img
                                :src="state.previewImage.dataUrl"
                                alt="大图预览"
                                class="max-w-full max-h-[70vh] object-contain rounded-lg shadow-md"
                            />
                        </div>
                        <button
                            class="absolute top-1/2 -translate-y-1/2 right-2.5 bg-white/80 border-none rounded-full w-10 h-10 flex items-center justify-center cursor-pointer z-10 shadow-md hover:bg-white hover:shadow-lg transition-all"
                            @click.stop="nextImage"
                        >
                            <svg
                                xmlns="http://www.w3.org/2000/svg"
                                width="24"
                                height="24"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <polyline points="9 18 15 12 9 6"></polyline>
                            </svg>
                        </button>
                    </div>
                    <div class="w-full text-center mt-4">
                        <p class="text-slate-700">
                            {{ state.previewImage.relativePath }}
                        </p>
                        <p class="text-sm text-slate-500 mt-2">
                            <span class="mx-2"
                                >大小:
                                {{ state.previewImage.formattedSize }}</span
                            >
                            <span class="mx-2"
                                >尺寸: {{ state.previewImage.resolution }}</span
                            >
                            <span class="mx-2"
                                >创建时间:
                                {{ state.previewImage.createdDate }}</span
                            >
                            <span class="mx-2"
                                >修改时间:
                                {{ state.previewImage.modifiedDate }}</span
                            >
                            <span class="mx-2"
                                >位置: {{ state.currentImageIndex + 1 }}/{{
                                    state.duplicateGroups[
                                        state.currentGroupIndex
                                    ]?.images.length
                                }}</span
                            >
                        </p>
                    </div>
                </div>
                <div
                    class="p-4 border-t border-slate-200 flex justify-between items-center"
                >
                    <div class="flex items-center gap-2">
                        <input
                            type="checkbox"
                            :id="`preview-img-checkbox`"
                            :checked="
                                isImageSelected(
                                    state.currentGroupIndex,
                                    state.currentImageIndex,
                                )
                            "
                            @change="toggleCurrentImageSelection"
                            class="rounded border-slate-300 text-blue-600 focus:ring-blue-500"
                        />
                        <label
                            for="preview-img-checkbox"
                            class="text-sm text-slate-700"
                            >选择此图片</label
                        >
                    </div>
                    <div class="flex gap-4">
                        <button
                            class="inline-flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg font-medium transition-all hover:bg-blue-700"
                            @click="openImage(state.previewImage.path)"
                        >
                            <svg
                                xmlns="http://www.w3.org/2000/svg"
                                width="16"
                                height="16"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <path
                                    d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"
                                />
                                <polyline points="15 3 21 3 21 9" />
                                <line x1="10" y1="14" x2="21" y2="3" />
                            </svg>
                            在系统中打开
                        </button>
                        <button
                            class="inline-flex items-center gap-2 px-4 py-2 bg-red-500 text-white rounded-lg font-medium transition-all hover:bg-red-600"
                            @click="deleteCurrentImage"
                        >
                            <svg
                                xmlns="http://www.w3.org/2000/svg"
                                width="16"
                                height="16"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <path d="M3 6h18" />
                                <path
                                    d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"
                                />
                                <path
                                    d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
                                />
                            </svg>
                            删除此图片
                        </button>
                    </div>
                </div>
            </div>
        </div>
        <!-- 顶部/底部导航按钮 -->
        <div class="fixed right-6 bottom-6 z-40">
            <div class="flex flex-col gap-3 items-center">
                <transition 
                   enter-active-class="transition ease-out duration-300 transform"
                   enter-from-class="opacity-0 translate-y-2 scale-95"
                   enter-to-class="opacity-100 translate-y-0 scale-100"
                   leave-active-class="transition ease-in duration-300 transform"
                   leave-from-class="opacity-100 translate-y-0 scale-100"
                   leave-to-class="opacity-0 translate-y-2 scale-95"
                >
                   <button 
                       v-if="state.showBackToTop && !groupNavVisible"
                       @click="scrollToTop"
                       class="bg-white w-12 h-12 rounded-full shadow-lg flex items-center justify-center text-slate-600 hover:bg-slate-50 hover:text-blue-600 transform hover:scale-110 active:scale-95 transition-all duration-300"
                       title="返回顶部"
                       aria-label="返回顶部"
                   >
                       <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                           <path d="m18 15-6-6-6 6"/>
                       </svg>
                   </button>
                </transition>
                <transition 
                    enter-active-class="transition ease-out duration-300 transform"
                    enter-from-class="opacity-0 translate-y-2 scale-95"
                    enter-to-class="opacity-100 translate-y-0 scale-100"
                    leave-active-class="transition ease-in duration-300 transform"
                    leave-from-class="opacity-100 translate-y-0 scale-100"
                    leave-to-class="opacity-0 translate-y-2 scale-95"
                >
                    <button 
                        v-if="(!isAtBottom || isHoveringBottomNav) && state.duplicateGroups.length > 0 && !groupNavVisible"
                        @click="scrollToBottom"
                        class="bg-white w-12 h-12 rounded-full shadow-lg flex items-center justify-center text-slate-600 hover:bg-slate-50 hover:text-blue-600 transform hover:scale-110 active:scale-95 transition-all duration-300"
                        title="滚动到底部"
                        aria-label="滚动到底部"
                        @mouseenter="isHoveringBottomNav = true"
                        @mouseleave="setTimeout(() => { isHoveringBottomNav = false }, 300)"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="m6 9 6 6 6-6"/>
                        </svg>
                    </button>
                </transition>
            </div>
        </div>
        
        <!-- 分组悬浮导航（滚动时显示） -->
        <div
            v-if="state.duplicateGroups.length > 3" 
            class="fixed left-6 top-1/2 -translate-y-1/2 z-40"
            @mouseenter="showGroupNav"
            @mouseleave="hideGroupNav"
        >
            <div v-if="!groupNavVisible && state.showGroupNav" 
                class="bg-white rounded-full shadow-xl p-3 border border-slate-200 cursor-pointer hover:bg-blue-50 transition-all duration-300 transform hover:scale-110 active:scale-95 animate-pulse" 
                @click="showGroupNav"
                title="打开分组导航菜单">
                <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-blue-600">
                    <path d="M3 9h18v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V9Z"></path>
                    <path d="m3 9 2.45-4.9A2 2 0 0 1 7.24 3h9.52a2 2 0 0 1 1.8 1.1L21 9"></path>
                    <path d="M12 3v6"></path>
                </svg>
            </div>
            <div v-else-if="groupNavVisible" 
                 class="bg-white rounded-xl shadow-xl p-4 border border-slate-200 transition-all duration-300"
                 :class="{'animate-slide-in': groupNavVisible}">
                <h3 class="text-sm font-medium text-slate-700 mb-3 px-1 flex items-center justify-between">
                    <span>分组导航</span> 
                    <span class="text-xs bg-blue-50 text-blue-600 px-2 py-0.5 rounded-full">共{{ state.duplicateGroups.length }}组</span>
                </h3>
                
                <div class="overflow-y-auto scrollbar-thin p-1" 
                     :style="{ 
                         maxHeight: state.duplicateGroups.length > 30 ? '350px' : 
                                  state.duplicateGroups.length > 15 ? '300px' : 'auto' 
                     }">
                    <!-- 分组网格布局 - 根据分组数量动态调整列数 -->
                    <div class="grid gap-2 mb-1"
                         :class="{
                             'grid-cols-3': state.duplicateGroups.length <= 12,
                             'grid-cols-4': state.duplicateGroups.length > 12 && state.duplicateGroups.length <= 50,
                             'grid-cols-5': state.duplicateGroups.length > 50
                         }">
                        <button
                            v-for="(group, index) in state.duplicateGroups"
                            :key="index"
                            @click="scrollToGroup(index)"
                            :class="{
                                'px-2 py-1.5 text-sm rounded-lg transition-all duration-200 flex items-center justify-center transform hover:scale-105': true,
                                'bg-blue-100 text-blue-700 font-medium': index === state.currentGroupIndex,
                                'bg-slate-100 hover:bg-blue-100 hover:text-blue-700 text-slate-700': index !== state.currentGroupIndex
                            }"
                            :title="`跳转到分组 #${index + 1} (${group.images.length}张图片)`"
                        >
                            {{ index + 1 }}
                        </button>
                    </div>
                </div>
                
                <!-- 底部操作栏 -->
                <div class="flex justify-between items-center mt-3 pt-3 border-t border-slate-200">
                    <div class="flex gap-2">
                        <button
                            @click="scrollToTop"
                            class="px-2 py-1 text-xs bg-blue-50 text-blue-600 hover:bg-blue-100 rounded-md flex items-center gap-1 shadow-sm transition-all hover:shadow"
                            title="返回顶部"
                        >
                            <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <path d="m18 15-6-6-6 6"/>
                            </svg>
                            顶部
                        </button>
                        <button
                            @click="scrollToBottom"
                            class="px-2 py-1 text-xs bg-blue-50 text-blue-600 hover:bg-blue-100 rounded-md flex items-center gap-1 shadow-sm transition-all hover:shadow"
                            title="滚动到底部"
                        >
                            底部
                            <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <path d="m6 9 6 6 6-6"/>
                            </svg>
                        </button>
                    </div>
                    <button 
                        @click="hideGroupNav" 
                        class="p-1 text-xs bg-slate-100 text-slate-600 hover:bg-slate-200 rounded-md flex items-center shadow-sm"
                        title="关闭导航"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <line x1="18" y1="6" x2="6" y2="18"></line>
                            <line x1="6" y1="6" x2="18" y2="18"></line>
                        </svg>
                    </button>
                </div>
            </div>
        </div>
    </main>
</template>
