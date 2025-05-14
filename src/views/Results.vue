<script setup>
import { ref, onMounted, computed, watch, inject } from "vue";
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
});

// 计算属性
const selectedCount = computed(() => {
    return Object.values(state.value.selectedImages).reduce((count, group) => {
        return count + Object.values(group).filter(Boolean).length;
    }, 0);
});

// 初始化组件
onMounted(() => {
    loadData();
});

// 监听路由参数变化
watch(
    () => route.params.timestamp,
    () => {
        loadData();
    },
);

// 加载数据
const loadData = async () => {
    try {
        if (!globalState.duplicateGroups?.length) {
            state.value.processingStatus =
                "没有找到重复图片，或所有重复图片已处理完毕";
            return;
        }

        state.value.duplicateGroups = globalState.duplicateGroups
            .map(processGroup)
            .filter(Boolean);
        state.value.selectedFolders = globalState.selectedFolders || [];

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
        state.value.processingStatus = `处理完成，共找到 ${totalDuplicates} 张存在重复的图片，分为 ${state.value.duplicateGroups.length} 组`;
    } catch (err) {
        console.error("加载数据失败:", err);
        state.value.processingStatus = "加载数据失败，请重试";
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
.animate-fade-in {
    animation: fadeIn 0.5s ease forwards;
}
</style>

<template>
    <main class="p-8 max-w-7xl mx-auto">
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
                    <p class="text-sm text-slate-600">
                        {{ state.processingStatus }}
                    </p>
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

            <!-- 显示重复图片组 -->
            <div
                v-if="state.duplicateGroups.length > 0"
                class="mt-4 animate-fade-in"
            >
                <div class="flex flex-col gap-8">
                    <div
                        v-for="(group, groupIndex) in state.duplicateGroups"
                        :key="groupIndex"
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
                            class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 p-6"
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
    </main>
</template>
