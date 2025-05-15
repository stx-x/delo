<script setup>
import { ref, inject, onMounted, computed } from "vue";
import { useRouter } from "vue-router";
import { open } from "@tauri-apps/plugin-dialog";

const router = useRouter();
const globalState = inject("globalState");

// 状态管理
const selectedFolders = ref([]);
const processingStatus = ref("");
const similarityThreshold = ref(80);
const algorithm = ref("Exact");
const recursive = ref(true);
const loading = ref(false);
const showAdvanced = ref(false);
const invoke = window.__TAURI__.core.invoke;

// 计算是否为精确哈希算法
// 检查是否为精确算法
const isExactAlgorithm = computed(() => algorithm.value === "Exact");

// 初始化状态
onMounted(() => {
    if (globalState.algorithm) {
        algorithm.value = globalState.algorithm;
    }
    if (globalState.similarityThreshold) {
        similarityThreshold.value = globalState.similarityThreshold;
    }
    if (typeof globalState.recursive !== "undefined") {
        recursive.value = globalState.recursive;
    }
});

// 路径格式化
const formatPath = (path) => {
    if (!path) return "";
    const parts = path.split(/[/\\]/);
    return parts.length > 2 ? `.../${parts.slice(-2).join("/")}` : path;
};

// 选择文件夹
const openFolderDialog = async () => {
    loading.value = true;
    processingStatus.value = "正在打开文件选择器...";

    try {
        const selected = await open({
            directory: true,
            multiple: true,
            title: "选择文件夹（按住 Ctrl/Command 键可选择多个）",
        });

        if (Array.isArray(selected)) {
            const newFolders = selected.filter(
                (folder) => !selectedFolders.value.includes(folder),
            );
            selectedFolders.value = [...selectedFolders.value, ...newFolders];
            processingStatus.value = `已添加 ${newFolders.length} 个文件夹，共选择 ${selectedFolders.value.length} 个文件夹`;
        } else if (selected === null) {
            console.log("用户取消了选择");
            processingStatus.value =
                selectedFolders.value.length > 0
                    ? `当前已选择 ${selectedFolders.value.length} 个文件夹`
                    : "";
        } else if (selected) {
            if (!selectedFolders.value.includes(selected)) {
                selectedFolders.value = [...selectedFolders.value, selected];
                processingStatus.value = `已添加文件夹，共选择 ${selectedFolders.value.length} 个文件夹`;
            }
        }
    } catch (err) {
        console.error("选择文件夹时出错：", err);
        processingStatus.value = `选择文件夹时出错: ${err.message || err}`;
    } finally {
        loading.value = false;
    }
};

// 移除文件夹
const removeFolder = (index) => {
    selectedFolders.value = selectedFolders.value.filter((_, i) => i !== index);
    processingStatus.value =
        selectedFolders.value.length > 0
            ? `当前已选择 ${selectedFolders.value.length} 个文件夹`
            : "";
};

// 清除选择
const clearSelection = () => {
    selectedFolders.value = [];
    processingStatus.value = "";
};

// 开始图片处理
const startProcessing = async () => {
    if (selectedFolders.value.length === 0) {
        processingStatus.value = "请先选择文件夹";
        return;
    }

    loading.value = true;
    processingStatus.value = "正在扫描图片...";

    try {
        // 首先获取总的文件和文件夹统计信息
        const scanStats = {
            totalFolders: 0,  // 将包括子文件夹总数
            totalFiles: 0,
            totalImages: 0,
            processedImages: 0,
        };

        // 获取总文件和图像数
        for (const folder of selectedFolders.value) {
            try {
                processingStatus.value = `正在统计文件夹 ${formatPath(folder)} 中的文件...`;
                const stats = await invoke("get_folder_stats", {
                    folder_path: folder,
                    recursive: recursive.value
                });
                
                if (stats) {
                    scanStats.totalFolders += stats.folder_count || 0;
                    scanStats.totalFiles += stats.total_files || 0;
                    scanStats.totalImages += stats.image_count || 0;
                }
            } catch (e) {
                console.warn(`获取文件夹 ${folder} 统计信息失败:`, e);
            }
        }

        processingStatus.value = `找到 ${scanStats.totalFiles} 个文件，其中 ${scanStats.totalImages} 张图片，正在查找重复...`;

        // 准备请求参数并包装在req对象中
        const duplicateGroups = await invoke("find_duplicates", {
            req: {
                folder_paths: selectedFolders.value,
                algorithm: algorithm.value,
                similarity_threshold: isExactAlgorithm.value ? 100 : Number(similarityThreshold.value),
                recursive: recursive.value,
            },
        });

        // 更新处理的图像数量
        if (duplicateGroups && duplicateGroups.length > 0) {
            scanStats.processedImages = duplicateGroups.reduce(
                (sum, group) => sum + (group.images ? group.images.length : 0),
                0
            );
        }

        // 保存到全局状态
        globalState.duplicateGroups = duplicateGroups;
        globalState.selectedFolders = [...selectedFolders.value];
        globalState.algorithm = algorithm.value;
        globalState.similarityThreshold = similarityThreshold.value;
        globalState.recursive = recursive.value;
        globalState.scanStats = scanStats;

        // 导航到结果页
        router.push({
            name: "results",
            params: {
                timestamp: Date.now().toString(),
            },
            replace: true,
        });
    } catch (err) {
        console.error("处理时出错：", err);
        processingStatus.value = `处理时出错: ${err.message || err}`;
        loading.value = false;
    }
};

// 算法选项
const algorithms = [
    { id: "Exact", name: "精确匹配", description: "仅查找完全相同的图片" },
    {
        id: "Perceptual",
        name: "感知哈希",
        description: "查找视觉上相似的图片，适合大多数场景",
    },
    {
        id: "Average",
        name: "均值哈希",
        description: "基于亮度平均值的简单算法",
    },
    { id: "Difference", name: "差异哈希", description: "基于亮度差异的算法" },
    {
        id: "ORB",
        name: "ORB特征",
        description: "基于特征点的高级算法，适合复杂图像",
    },
];

// 切换高级选项
const toggleAdvanced = () => {
    showAdvanced.value = !showAdvanced.value;
};
</script>

<template>
    <main
        class="min-h-screen flex flex-col items-center justify-center p-6 bg-gradient-to-br from-slate-50 to-blue-50"
    >
        <div class="w-full max-w-4xl">
            <!-- 品牌标题 -->
            <div class="text-center mb-12">
                <div class="relative inline-block">
                    <!-- 装饰元素 -->
                    <div class="absolute -top-8 -left-8 w-16 h-16 bg-gradient-to-br from-blue-400 to-indigo-500 rounded-full opacity-20 animate-pulse-slow"></div>
                    <div class="absolute -bottom-6 -right-6 w-12 h-12 bg-gradient-to-br from-indigo-400 to-purple-500 rounded-full opacity-20 animate-pulse-slow" style="animation-delay: 1s;"></div>
                    
                    <h1 class="flex items-center justify-center gap-3 text-7xl font-black tracking-tight text-center relative">
                        <div class="brand-wrapper px-8 py-3 rounded-2xl">
                            <span class="text-transparent bg-clip-text bg-gradient-to-r from-blue-600 via-indigo-500 to-blue-700 hover:from-blue-700 hover:via-indigo-600 hover:to-blue-800 transition-all duration-500 brand-text drop-shadow-lg">Delo</span>
                            <span class="text-indigo-300 mx-1.5 brand-dot">·</span>
                            <span class="font-art text-transparent bg-clip-text bg-gradient-to-r from-blue-500 via-indigo-600 to-blue-700 hover:from-blue-600 hover:via-indigo-700 hover:to-blue-800 transition-all duration-500 brand-text-cn drop-shadow-lg">
                                去若
                            </span>
                        </div>
                    </h1>
                </div>
                
                <p class="text-slate-600 mt-8 text-xl font-medium tracking-wide brand-slogan max-w-2xl mx-auto">
                    <span class="inline-block bg-gradient-to-r from-blue-50 to-indigo-50 px-4 py-2 rounded-xl shadow-sm">
                        快速识别并管理重复图片，让您的相册
                        <span class="text-blue-600 font-semibold inline-block transform hover:scale-105 transition-transform duration-300">井然有序</span>
                    </span>
                </p>
            </div>

            <!-- 无选择文件夹状态 -->
            <div
                v-if="selectedFolders.length === 0"
                class="flex flex-col items-center gap-6"
            >
                <button
                    @click="openFolderDialog"
                    class="flex items-center gap-2 px-8 py-4 bg-blue-600 text-white rounded-xl font-semibold text-lg shadow-lg hover:shadow-xl transform transition hover:-translate-y-1 active:translate-y-0 disabled:opacity-70"
                    :disabled="loading"
                >
                    <svg
                        v-if="loading"
                        class="animate-spin h-5 w-5"
                        xmlns="http://www.w3.org/2000/svg"
                        fill="none"
                        viewBox="0 0 24 24"
                    >
                        <circle
                            class="opacity-25"
                            cx="12"
                            cy="12"
                            r="10"
                            stroke="currentColor"
                            stroke-width="4"
                        ></circle>
                        <path
                            class="opacity-75"
                            fill="currentColor"
                            d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                        ></path>
                    </svg>
                    <svg
                        v-else
                        xmlns="http://www.w3.org/2000/svg"
                        class="h-6 w-6"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                    >
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            stroke-width="2"
                            d="M9 13h6m-3-3v6m5 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                        />
                    </svg>
                    {{ loading ? "处理中..." : "选择文件夹" }}
                </button>
            </div>

            <!-- 已选择文件夹状态 -->
            <div v-else class="flex flex-col gap-6 animate-fade-in w-full">
                <!-- 文件夹列表区域 -->
                <div
                    class="bg-gradient-to-br from-white to-blue-50 rounded-2xl shadow-lg p-6 transition-all hover:shadow-xl border border-blue-100"
                >
                    <div class="flex justify-between items-center mb-6">
                        <h2
                            class="text-xl font-bold text-blue-800 flex items-center"
                        >
                            <div class="bg-blue-100 p-2 rounded-lg mr-3">
                                <svg
                                    xmlns="http://www.w3.org/2000/svg"
                                    class="h-5 w-5 text-blue-600"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke="currentColor"
                                >
                                    <path
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        stroke-width="2"
                                        d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
                                    />
                                    <path
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        stroke-width="2"
                                        d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                                    />
                                </svg>
                            </div>
                            <span class="relative">
                                检测设置
                                <div class="absolute -bottom-1 left-0 right-0 h-0.5 bg-gradient-to-r from-transparent via-blue-400 to-transparent opacity-50"></div>
                            </span>
                        </h2>

                        <button
                            @click="toggleAdvanced"
                            class="px-4 py-2.5 text-sm bg-gradient-to-r from-blue-50 to-indigo-50 hover:from-blue-100 hover:to-indigo-100 text-blue-700 rounded-xl transition-all flex items-center gap-2 shadow-sm hover:shadow transform hover:-translate-y-0.5 active:translate-y-0 border border-blue-100"
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
                                class="w-4 h-4 text-blue-600"
                            >
                                <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
                                <line x1="9" y1="12" x2="15" y2="12"></line>
                                <line x1="12" y1="9" x2="12" y2="15"></line>
                            </svg>
                            {{ showAdvanced ? "隐藏更多选项" : "显示更多选项" }}
                            <span
                                class="inline-flex items-center justify-center w-5 h-5 ml-1 bg-blue-100 text-blue-700 rounded-full text-xs font-bold"
                            >
                                {{ showAdvanced ? "-" : "+" }}
                            </span>
                        </button>
                    </div>

                    <!-- 算法选择 -->
                    <div class="mb-6">
                        <label
                            class="block text-lg font-semibold text-blue-800 mb-4 flex items-center gap-2 group"
                        >
                            <div class="bg-blue-100 p-2 rounded-lg">
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
                                    class="w-5 h-5 text-blue-600 group-hover:text-blue-700 transition-colors"
                                >
                                    <path
                                        d="M14.5 4h-5L7 7H4a2 2 0 00-2 2v9a2 2 0 002 2h16a2 2 0 002-2V9a2 2 0 00-2-2h-3l-2.5-3z"
                                    ></path>
                                    <circle cx="12" cy="13" r="3"></circle>
                                </svg>
                            </div>
                            <span class="relative">
                                检测算法
                                <div class="absolute -bottom-1 left-0 right-0 h-0.5 bg-gradient-to-r from-transparent via-blue-400 to-transparent opacity-50"></div>
                            </span>
                        </label>
                        <div
                            class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 mb-2"
                        >
                            <div
                                v-for="algo in showAdvanced
                                    ? algorithms
                                    : algorithms.filter((a) =>
                                          [
                                              'Exact',
                                              'Perceptual',
                                              'ORB',
                                          ].includes(a.id),
                                      )"
                                :key="algo.id"
                                @click="algorithm = algo.id"
                                class="flex items-start p-5 rounded-xl border cursor-pointer transition-all duration-300 transform hover:scale-102 hover:shadow-md group relative overflow-hidden"
                                :class="
                                    algorithm === algo.id
                                        ? 'border-blue-500 bg-gradient-to-br from-blue-50 to-indigo-50 shadow-md ring-1 ring-blue-200'
                                        : 'border-slate-200 hover:border-blue-300 bg-white'
                                "
                            >
                                <!-- 背景装饰 -->
                                <div v-if="algorithm === algo.id" class="absolute -right-4 -bottom-4 w-16 h-16 bg-blue-100 rounded-full opacity-30"></div>
                                <div class="flex-shrink-0 mr-3 mt-0.5">
                                    <div
                                        class="w-5 h-5 rounded-full flex items-center justify-center transition-all duration-300 border-2"
                                        :class="
                                            algorithm === algo.id
                                                ? 'border-blue-500 bg-blue-500'
                                                : 'border-slate-300 group-hover:border-blue-300'
                                        "
                                    >
                                        <svg
                                            v-if="algorithm === algo.id"
                                            xmlns="http://www.w3.org/2000/svg"
                                            class="h-3 w-3 text-white"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </div>
                                </div>
                                <div>
                                    <div
                                        class="font-medium text-lg text-slate-800 mb-1.5 group-hover:text-blue-700 transition-colors"
                                    >
                                        {{ algo.name }}
                                        <span
                                            v-if="algorithm === algo.id"
                                            class="inline-block ml-1.5 text-xs font-normal px-1.5 py-0.5 bg-blue-100 text-blue-700 rounded-full animate-fadeIn"
                                            >当前选择</span
                                        >
                                    </div>
                                    <p
                                        class="text-sm text-slate-500 group-hover:text-slate-600 leading-relaxed"
                                    >
                                        {{ algo.description }}
                                    </p>
                                </div>
                            </div>
                        </div>
                    </div>

                    <!-- 相似度阈值 -->
                    <div class="mb-5" :class="{'opacity-50': isExactAlgorithm}">
                        <label
                            class="block text-lg font-semibold text-slate-800 mb-3 flex items-center gap-2 group"
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
                                class="w-5 h-5 text-blue-600 group-hover:text-blue-700 transition-colors"
                            >
                                <path d="M5 12h14"></path>
                                <path d="M12 5v14"></path>
                            </svg>
                            <span class="relative">
                                相似度阈值
                                <span
                                    class="absolute -bottom-1 left-0 w-full h-0.5 bg-gradient-to-r from-blue-500 to-transparent transform scale-x-0 group-hover:scale-x-100 transition-transform origin-left"
                                ></span>
                            </span>
                            <span
                                class="ml-2 px-2.5 py-1 bg-gradient-to-r from-blue-500 to-blue-600 text-white text-sm rounded-full font-bold shadow-sm transform transition-transform hover:scale-105"
                            >
                                {{ similarityThreshold }}%
                            </span>
                        </label>
                        <div
                            class="bg-slate-50 p-4 rounded-lg border border-slate-200"
                        >
                            <input
                                type="range"
                                v-model="similarityThreshold"
                                min="0"
                                max="100"
                                step="1"
                                class="w-full h-2 bg-slate-200 rounded-lg appearance-none cursor-pointer"
                                :disabled="isExactAlgorithm"
                            />
                            <div
                                class="flex justify-between text-sm text-slate-600 mt-3 font-medium"
                            >
                                <span
                                    class="flex items-center gap-1.5 bg-slate-100 px-3 py-1 rounded-lg transition-all hover:bg-slate-200"
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
                                        class="text-amber-500"
                                    >
                                        <circle cx="12" cy="12" r="10"></circle>
                                        <line
                                            x1="15"
                                            y1="9"
                                            x2="9"
                                            y2="15"
                                        ></line>
                                        <line
                                            x1="9"
                                            y1="9"
                                            x2="15"
                                            y2="15"
                                        ></line>
                                    </svg>
                                    宽松检测
                                    <span class="text-xs text-slate-500 ml-1"
                                        >(找到更多可能相似的图片)</span
                                    >
                                </span>
                                <span
                                    class="flex items-center gap-1.5 bg-slate-100 px-3 py-1 rounded-lg transition-all hover:bg-slate-200"
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
                                        class="text-green-500"
                                    >
                                        <path
                                            d="M22 11.08V12a10 10 0 1 1-5.93-9.14"
                                        ></path>
                                        <polyline
                                            points="22 4 12 14.01 9 11.01"
                                        ></polyline>
                                    </svg>
                                    严格检测
                                    <span class="text-xs text-slate-500 ml-1"
                                        >(只找出高度相似的图片)</span
                                    >
                                </span>
                            </div>
                        </div>
                    </div>

                    <!-- 递归选项 -->
                    <div
                        v-if="showAdvanced"
                        class="flex items-center p-3 bg-slate-50 rounded-lg border border-slate-200"
                    >
                        <input
                            type="checkbox"
                            id="recursive-option"
                            v-model="recursive"
                            class="w-5 h-5 text-blue-600 border-slate-300 rounded focus:ring-blue-500 focus:ring-2 transition-all"
                        />
                        <label
                            for="recursive-option"
                            class="ml-2 text-md text-slate-800 flex items-center gap-2 cursor-pointer hover:text-blue-700 transition-colors"
                        >
                            <svg
                                xmlns="http://www.w3.org/2000/svg"
                                width="18"
                                height="18"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                class="w-5 h-5 text-blue-600"
                            >
                                <path
                                    d="M21 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h6"
                                ></path>
                                <path d="m21 3-9 9"></path>
                                <path d="M15 3h6v6"></path>
                            </svg>
                            包含子文件夹中的图片
                            <span
                                class="px-1.5 py-0.5 bg-blue-100 text-blue-600 text-xs rounded-md"
                                >递归搜索</span
                            >
                        </label>
                    </div>
                </div>
            </div>

            <!-- 已选择文件夹状态 -->
            <div v-else class="flex flex-col gap-6 animate-fade-in w-full">
                <!-- 文件夹列表区域 -->
                <div
                    class="bg-gradient-to-br from-white to-blue-50 rounded-2xl shadow-lg p-6 transition-all hover:shadow-xl border border-blue-100"
                >
                    <div class="flex justify-between items-center mb-6">
                        <h2
                            class="text-xl font-bold text-blue-800 flex items-center"
                        >
                            <div class="bg-blue-100 p-2 rounded-lg mr-3">
                                <svg
                                    xmlns="http://www.w3.org/2000/svg"
                                    class="h-5 w-5 text-blue-600"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke="currentColor"
                                >
                                    <path
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        stroke-width="2"
                                        d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
                                    />
                                </svg>
                            </div>
                            <span class="relative">
                                已选择的文件夹
                                <div class="absolute -bottom-1 left-0 right-0 h-0.5 bg-gradient-to-r from-transparent via-blue-400 to-transparent opacity-50"></div>
                            </span>
                            <span
                                class="ml-3 px-2.5 py-1 bg-blue-100 text-blue-700 text-sm rounded-full font-medium"
                                >{{ selectedFolders.length }}个</span
                            >
                        </h2>

                        <div class="flex gap-3">
                            <button
                                @click="openFolderDialog"
                                class="px-4 py-2 text-sm bg-gradient-to-r from-blue-500 to-blue-600 hover:from-blue-600 hover:to-blue-700 text-white rounded-lg shadow-md hover:shadow-lg transition-all duration-300 ease-in-out flex items-center transform hover:-translate-y-0.5 hover:scale-105 active:translate-y-0 active:scale-100 relative overflow-hidden group"
                                :disabled="loading"
                            >
                                <span class="absolute inset-0 w-full h-full bg-gradient-to-r from-blue-400 to-blue-500 opacity-0 group-hover:opacity-20 transform scale-x-0 group-hover:scale-x-100 transition-all duration-300 ease-out origin-left"></span>
                                <svg
                                    xmlns="http://www.w3.org/2000/svg"
                                    class="h-4 w-4 mr-2 transition-transform duration-300 group-hover:rotate-90 group-hover:scale-110"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke="currentColor"
                                >
                                    <path
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        stroke-width="2"
                                        d="M12 6v6m0 0v6m0-6h6m-6 0H6"
                                    />
                                </svg>
                                <span class="relative z-10">添加文件夹</span>
                            </button>

                            <button
                                @click="clearSelection"
                                class="px-4 py-2 text-sm bg-gradient-to-r from-red-50 to-red-100 hover:from-red-100 hover:to-red-200 text-red-600 hover:text-red-700 rounded-lg shadow-sm hover:shadow transition-all duration-300 ease-in-out flex items-center transform hover:-translate-y-0.5 hover:scale-105 active:translate-y-0 active:scale-100 border border-red-200 relative overflow-hidden group"
                            >
                                <span class="absolute inset-0 w-full h-full bg-red-200 opacity-0 group-hover:opacity-20 transform scale-x-0 group-hover:scale-x-100 transition-all duration-300 ease-out origin-left rounded-lg"></span>
                                <svg
                                    xmlns="http://www.w3.org/2000/svg"
                                    class="h-4 w-4 mr-2 transition-all duration-300 group-hover:scale-110 group-hover:text-red-700"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke="currentColor"
                                >
                                    <path
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        stroke-width="2"
                                        d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                                    />
                                </svg>
                                <span class="relative z-10">清除全部</span>
                            </button>
                        </div>
                    </div>

                    <!-- 文件夹列表 -->
                    <div
                        class="bg-white rounded-xl border border-blue-100 max-h-[250px] overflow-y-auto shadow-inner p-2"
                    >
                        <ul class="divide-y divide-blue-50">
                            <li
                                v-for="(folder, index) in selectedFolders"
                                :key="index"
                                class="p-3 flex justify-between items-center hover:bg-gradient-to-r hover:from-blue-50 hover:to-indigo-50 transition-all duration-300 group rounded-lg"
                            >
                                <div class="flex items-center flex-1 min-w-0">
                                    <div
                                        class="flex-shrink-0 w-10 h-10 bg-gradient-to-br from-blue-100 to-blue-200 rounded-lg flex items-center justify-center mr-4 group-hover:from-blue-200 group-hover:to-blue-300 transition-colors shadow-sm"
                                    >
                                        <svg
                                            xmlns="http://www.w3.org/2000/svg"
                                            class="h-5 w-5 text-blue-600 group-hover:text-blue-700 transition-colors"
                                            fill="none"
                                            viewBox="0 0 24 24"
                                            stroke="currentColor"
                                        >
                                            <path
                                                stroke-linecap="round"
                                                stroke-linejoin="round"
                                                stroke-width="2"
                                                d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
                                            />
                                        </svg>
                                    </div>
                                    <div class="flex-1 min-w-0">
                                        <p
                                            class="text-sm font-medium text-slate-800 truncate group-hover:text-blue-700 transition-colors"
                                        >
                                            {{ formatPath(folder) }}
                                        </p>
                                        <p
                                            class="text-xs text-slate-500 truncate group-hover:text-slate-600 transition-colors"
                                        >
                                            {{ folder }}
                                        </p>
                                    </div>
                                </div>
                                <button
                                    @click="removeFolder(index)"
                                    class="ml-2 p-2 text-slate-400 hover:text-red-500 hover:bg-red-50 rounded-lg transition-all transform hover:scale-110 active:scale-95"
                                >
                                    <svg
                                        xmlns="http://www.w3.org/2000/svg"
                                        class="h-4 w-4"
                                        fill="none"
                                        viewBox="0 0 24 24"
                                        stroke="currentColor"
                                    >
                                        <path
                                            stroke-linecap="round"
                                            stroke-linejoin="round"
                                            stroke-width="2"
                                            d="M6 18L18 6M6 6l12 12"
                                        />
                                    </svg>
                                </button>
                            </li>
                            <!-- 当没有文件夹时显示提示 -->
                            <li v-if="selectedFolders.length === 0" class="p-6 text-center">
                                <div class="text-slate-400 flex flex-col items-center">
                                    <svg
                                        xmlns="http://www.w3.org/2000/svg"
                                        class="h-10 w-10 mb-2"
                                        fill="none"
                                        viewBox="0 0 24 24"
                                        stroke="currentColor"
                                    >
                                        <path
                                            stroke-linecap="round"
                                            stroke-linejoin="round"
                                            stroke-width="1.5"
                                            d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4"
                                        />
                                    </svg>
                                    <p class="text-sm">请点击“添加文件夹”按钮选择要扫描的文件夹</p>
                                </div>
                            </li>
                        </ul>
                    </div>
                </div>

                <!-- 状态提示 -->
                <div
                    v-if="processingStatus"
                    class="p-4 bg-slate-50 rounded-lg text-sm animate-fade-in border border-slate-200"
                >
                    <p class="text-slate-700">{{ processingStatus }}</p>
                </div>

                <!-- 设置区域 -->
                <div
                    class="bg-white rounded-xl shadow-md p-6 transition-all hover:shadow-lg"
                >
                    <div class="flex justify-between items-center mb-4">
                        <h2
                            class="text-xl font-bold text-slate-800 flex items-center"
                        >
                            <svg
                                xmlns="http://www.w3.org/2000/svg"
                                class="h-6 w-6 text-blue-600"
                                fill="none"
                                viewBox="0 0 24 24"
                                stroke="currentColor"
                                title="检测设置"
                            >
                                <path
                                    stroke-linecap="round"
                                    stroke-linejoin="round"
                                    stroke-width="2"
                                    d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"
                                />
                            </svg>
                        </h2>

                        <button
                            @click="toggleAdvanced"
                            class="px-3 py-1.5 text-sm bg-slate-100 hover:bg-slate-200 text-slate-700 rounded-lg transition-all flex items-center gap-1"
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
                                class="w-4 h-4 text-blue-600"
                            >
                                <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
                                <line x1="9" y1="12" x2="15" y2="12"></line>
                                <line x1="12" y1="9" x2="12" y2="15"></line>
                            </svg>
                            {{ showAdvanced ? "隐藏更多选项" : "显示更多选项" }}
                        </button>
                    </div>

                    <!-- 算法选择 -->
                    <div class="mb-6">
                        <label
                            class="block text-sm font-medium text-slate-700 mb-3 flex items-center gap-2"
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
                                class="w-4 h-4 text-blue-600"
                            >
                                <path d="M5 18a7 7 0 1 0 10-8"></path>
                                <path d="M9 17h6v-5h-6z"></path>
                                <path d="M6 9h2"></path>
                                <path d="M4 6h2"></path>
                                <path d="M2 12h2"></path>
                                <path d="M12 4v4"></path>
                            </svg>
                            检测算法
                        </label>
                        <div
                            class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 mb-2"
                        >
                            <div
                                v-for="algo in showAdvanced
                                    ? algorithms
                                    : algorithms.filter((a) =>
                                          [
                                              'Exact',
                                              'Perceptual',
                                              'ORB',
                                          ].includes(a.id),
                                      )"
                                :key="algo.id"
                                @click="algorithm = algo.id"
                                class="flex items-start p-5 rounded-xl border cursor-pointer transition-all duration-300 transform hover:scale-102 hover:shadow-md group relative overflow-hidden"
                                :class="
                                    algorithm === algo.id
                                        ? 'border-blue-500 bg-gradient-to-br from-blue-50 to-indigo-50 shadow-md ring-1 ring-blue-200'
                                        : 'border-slate-200 hover:border-blue-300 bg-white'
                                "
                            >
                                <!-- 背景装饰 -->
                                <div v-if="algorithm === algo.id" class="absolute -right-4 -bottom-4 w-16 h-16 bg-blue-100 rounded-full opacity-30"></div>
                                <div class="flex-shrink-0 mr-3 mt-0.5">
                                    <div
                                        class="w-5 h-5 rounded-full flex items-center justify-center transition-all duration-300 border-2"
                                        :class="
                                            algorithm === algo.id
                                                ? 'border-blue-500 bg-blue-500'
                                                : 'border-slate-300 group-hover:border-blue-300'
                                        "
                                    >
                                        <svg
                                            v-if="algorithm === algo.id"
                                            xmlns="http://www.w3.org/2000/svg"
                                            class="h-3 w-3 text-white"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </div>
                                </div>
                                <div>
                                    <div
                                        class="font-medium text-slate-800 mb-1 group-hover:text-blue-700 transition-colors"
                                    >
                                        {{ algo.name }}
                                    </div>
                                    <p
                                        class="text-xs text-slate-500 group-hover:text-slate-600"
                                    >
                                        {{ algo.description }}
                                    </p>
                                </div>
                            </div>
                        </div>
                    </div>

                    <!-- 相似度阈值 -->
                    <div class="mb-5 pb-5 border-b border-slate-200" :class="{'opacity-50': isExactAlgorithm}">
                        <label
                            class="block mb-2 font-medium text-slate-800 flex items-center gap-2"
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
                                class="w-4 h-4 text-blue-600"
                            >
                                <path d="M5 12h14"></path>
                                <path d="M12 5v14"></path>
                            </svg>
                            相似度阈值
                            <span
                                class="ml-1 px-2 py-0.5 bg-blue-100 text-blue-700 text-sm rounded-full font-bold"
                            >
                                {{ similarityThreshold }}%
                            </span>
                        </label>
                        <div
                            class="bg-slate-50 p-4 rounded-lg border border-slate-200"
                        >
                            <input
                                type="range"
                                v-model="similarityThreshold"
                                min="0"
                                max="100"
                                step="1"
                                class="w-full h-2 bg-slate-200 rounded-lg appearance-none cursor-pointer"
                                :disabled="isExactAlgorithm"
                            />
                            <div
                                class="flex justify-between text-xs text-slate-500 mt-2 font-medium"
                            >
                                <span class="flex items-center gap-1">
                                    <svg
                                        xmlns="http://www.w3.org/2000/svg"
                                        width="14"
                                        height="14"
                                        viewBox="0 0 24 24"
                                        fill="none"
                                        stroke="currentColor"
                                        stroke-width="2"
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                    >
                                        <circle cx="12" cy="12" r="10"></circle>
                                        <line
                                            x1="15"
                                            y1="9"
                                            x2="9"
                                            y2="15"
                                        ></line>
                                        <line
                                            x1="9"
                                            y1="9"
                                            x2="15"
                                            y2="15"
                                        ></line>
                                    </svg>
                                    宽松 (低阈值)
                                </span>
                                <span class="flex items-center gap-1">
                                    <svg
                                        xmlns="http://www.w3.org/2000/svg"
                                        width="14"
                                        height="14"
                                        viewBox="0 0 24 24"
                                        fill="none"
                                        stroke="currentColor"
                                        stroke-width="2"
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                    >
                                        <path
                                            d="M22 11.08V12a10 10 0 1 1-5.93-9.14"
                                        ></path>
                                        <polyline
                                            points="22 4 12 14.01 9 11.01"
                                        ></polyline>
                                    </svg>
                                    严格 (高阈值)
                                </span>
                            </div>
                        </div>
                    </div>

                    <!-- 递归选项 -->
                    <div
                        v-if="showAdvanced"
                        class="flex items-center p-3 bg-slate-50 rounded-lg border border-slate-200"
                    >
                        <input
                            type="checkbox"
                            id="recursive-option"
                            v-model="recursive"
                            class="w-5 h-5 text-blue-600 border-slate-300 rounded focus:ring-blue-500 focus:ring-2 transition-all"
                        />
                        <label
                            for="recursive-option"
                            class="ml-2 text-sm text-slate-700 flex items-center gap-2"
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
                                class="w-4 h-4 text-blue-600"
                            >
                                <path
                                    d="M21 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h6"
                                ></path>
                                <path d="m21 3-9 9"></path>
                                <path d="M15 3h6v6"></path>
                            </svg>
                            包含子文件夹中的图片 (递归搜索)
                        </label>
                    </div>
                </div>

                <!-- 开始处理按钮 -->
                <div class="text-center">
                    <button
                        @click="startProcessing"
                        class="inline-flex items-center gap-2 px-8 py-4 bg-blue-600 text-white rounded-xl font-semibold shadow-lg hover:shadow-xl transform transition hover:-translate-y-1 active:translate-y-0 disabled:opacity-70"
                        :disabled="loading || selectedFolders.length === 0"
                    >
                        <svg
                            v-if="loading"
                            class="animate-spin h-5 w-5"
                            xmlns="http://www.w3.org/2000/svg"
                            fill="none"
                            viewBox="0 0 24 24"
                        >
                            <circle
                                class="opacity-25"
                                cx="12"
                                cy="12"
                                r="10"
                                stroke="currentColor"
                                stroke-width="4"
                            ></circle>
                            <path
                                class="opacity-75"
                                fill="currentColor"
                                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                            ></path>
                        </svg>
                        <svg
                            v-else
                            xmlns="http://www.w3.org/2000/svg"
                            class="h-5 w-5"
                            fill="none"
                            viewBox="0 0 24 24"
                            stroke="currentColor"
                        >
                            <path
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                stroke-width="2"
                                d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"
                            />
                            <path
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                stroke-width="2"
                                d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                            />
                        </svg>
                        {{ loading ? "处理中..." : "开始查找重复图片" }}
                    </button>
                </div>
            </div>
        </div>
    </main>
</template>

<style>
/* 动画效果 */
@keyframes float {
    0%,
    100% {
        transform: translateY(0);
    }
    50% {
        transform: translateY(-10px);
    }
}

@keyframes pulse-slow {
    0%,
    100% {
        opacity: 1;
    }
    50% {
        opacity: 0.8;
    }
}

/* 已移除动画关键帧 */

.brand-slogan {
    opacity: 0.9;
    transition: all 0.5s ease;
}

.brand-wrapper:hover + .brand-slogan {
    opacity: 1;
    transform: translateY(3px);
}

.animate-float {
    animation: float 6s ease-in-out infinite;
}

.hover\:scale-102:hover {
    transform: scale(1.02);
}

.animate-fade-in {
    animation: fadeIn 0.5s ease forwards;
}

.brand-wrapper {
    position: relative;
    display: flex;
    align-items: center;
    padding: 0.5rem 1.5rem;
    border-radius: 1rem;
    overflow: hidden;
    transition: all 0.7s ease;
}

.brand-wrapper:hover {
    background: rgba(59, 130, 246, 0.03);
    box-shadow: 0 0 25px rgba(59, 130, 246, 0.1);
}

.brand-wrapper:hover::before {
    opacity: 0.8;
    transform: translateY(0);
}

.brand-wrapper::before {
    content: "";
    position: absolute;
    bottom: 0;
    left: 10%;
    right: 10%;
    height: 2px;
    background: linear-gradient(
        90deg,
        transparent,
        rgba(59, 130, 246, 0.7),
        transparent
    );
    opacity: 0;
    transform: translateY(5px);
    transition: all 0.8s ease;
}

.brand-text {
    display: inline-block;
    transform-origin: bottom;
    transition: all 0.5s ease;
}

.brand-text-cn {
    display: inline-block;
    transform-origin: bottom;
    transition: all 0.5s ease;
}

.brand-wrapper:hover .brand-text {
    transform: scale(1.03);
    text-shadow: 0 10px 20px rgba(59, 130, 246, 0.3);
}

.brand-wrapper:hover .brand-text-cn {
    transform: scale(1.05) rotate(1deg);
    text-shadow: 0 10px 25px rgba(59, 130, 246, 0.4);
}

.brand-dot {
    display: inline-block;
    transition: all 0.7s ease;
}

.brand-wrapper:hover .brand-dot {
    transform: rotate(30deg) scale(1.1);
    color: #3b82f6;
}

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

/* 艺术风格字体 */
.font-art {
    font-family: "华文行楷", "STXingkai", "楷体", "KaiTi", cursive;
    font-weight: 700;
    letter-spacing: 0.05em;
    transform-origin: center;
    transform: scale(1.1) rotate(-2deg);
    transition: all 0.3s ease;
    position: relative;
}

.font-art:hover {
    text-shadow:
        0 0 12px rgba(59, 130, 246, 0.7),
        0 0 25px rgba(99, 102, 241, 0.5),
        0 0 40px rgba(79, 70, 229, 0.3);
}

.drop-shadow-art {
    filter: drop-shadow(0 0 8px rgba(59, 130, 246, 0.4));
}

/* 自定义滑块样式 */
input[type="range"] {
    -webkit-appearance: none;
    height: 6px;
    background: #e2e8f0;
    border-radius: 8px;
    outline: none;
}

input[type="range"]::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 18px;
    height: 18px;
    background: #3b82f6;
    border-radius: 50%;
    cursor: pointer;
    transition: all 0.2s ease;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

input[type="range"]::-webkit-slider-thumb:hover {
    transform: scale(1.1);
    background: #2563eb;
}

input[type="range"]::-moz-range-thumb {
    width: 18px;
    height: 18px;
    background: #3b82f6;
    border-radius: 50%;
    cursor: pointer;
    transition: all 0.2s ease;
    border: none;
}

input[type="range"]::-moz-range-thumb:hover {
    transform: scale(1.1);
    background: #2563eb;
}

/* 自定义滚动条 */
.overflow-y-auto {
    scrollbar-width: thin;
    scrollbar-color: #cbd5e1 #f1f5f9;
}

.overflow-y-auto::-webkit-scrollbar {
    width: 6px;
}

.overflow-y-auto::-webkit-scrollbar-track {
    background: #f1f5f9;
    border-radius: 3px;
}

.overflow-y-auto::-webkit-scrollbar-thumb {
    background-color: #cbd5e1;
    border-radius: 3px;
}

.overflow-y-auto::-webkit-scrollbar-thumb:hover {
    background-color: #94a3b8;
}
</style>
