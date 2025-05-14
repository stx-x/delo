<script setup>
import { ref, computed } from 'vue';

const props = defineProps({
  modelValue: {
    type: String,
    required: true
  },
  similarityThreshold: {
    type: Number,
    required: true
  }
});

const emit = defineEmits(['update:modelValue', 'update:similarityThreshold']);

// 算法数据 - 与后端HashAlgorithm枚举保持一致
const algorithms = [
  {
    id: 'Exact',
    name: '精确匹配',
    description: '使用SHA-256进行完全相同的图片检测',
    category: '精确匹配',
    threshold: false
  },
  {
    id: 'Average',
    name: '平均哈希',
    description: '计算图片平均亮度，对缩放和亮度变化敏感',
    category: '哈希算法',
    threshold: true
  },
  {
    id: 'Difference',
    name: '差异哈希',
    description: '比较相邻像素差异，对缩放和亮度变化敏感',
    category: '哈希算法',
    threshold: true
  },
  {
    id: 'Perceptual',
    name: '感知哈希',
    description: '使用DCT变换，对缩放、旋转和亮度变化更鲁棒',
    category: '哈希算法',
    threshold: true
  },
  {
    id: 'ORB',
    name: 'ORB特征',
    description: '定向FAST和旋转BRIEF，快速特征点检测和描述',
    category: '特征点匹配',
    threshold: true
  }
];

// 当前选中的算法
const currentAlgorithm = computed(() => {
  return algorithms.find(algo => algo.id === props.modelValue) || algorithms[0];
});

// 是否显示阈值滑块
const showThreshold = computed(() => {
  return currentAlgorithm.value?.threshold ?? false;
});

// 选择算法
const selectAlgorithm = (id) => {
  emit('update:modelValue', id);
};

// 更新阈值
const updateThreshold = (value) => {
  emit('update:similarityThreshold', value);
};

// 分组算法
const algorithmsByCategory = computed(() => {
  const grouped = {};
  algorithms.forEach(algo => {
    if (!grouped[algo.category]) {
      grouped[algo.category] = [];
    }
    grouped[algo.category].push(algo);
  });
  return grouped;
});
</script>

<template>
  <div class="w-full space-y-6">
    <!-- 算法选择 -->
    <div class="space-y-4">
      <div v-for="(algos, category) in algorithmsByCategory" :key="category" class="space-y-2">
        <div class="text-xs font-medium text-gray-500 uppercase tracking-wider">{{ category }}</div>
        <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
          <button
            v-for="algo in algos"
            :key="algo.id"
            @click="!algo.disabled && selectAlgorithm(algo.id)"
            class="relative flex items-center p-3 border rounded-md transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
            :class="[
              algo.id === modelValue 
                ? 'bg-blue-50 border-blue-300 text-blue-700' 
                : 'bg-white border-gray-200 text-gray-700 hover:bg-gray-50',
              algo.disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'
            ]"
            :disabled="algo.disabled"
          >
            <div class="flex-1 text-left">
              <div class="font-medium">{{ algo.name }}</div>
              <div class="text-xs text-gray-500 mt-1 line-clamp-2">{{ algo.description }}</div>
            </div>
            <div v-if="algo.id === modelValue" class="absolute top-1 right-1 w-2 h-2 bg-blue-500 rounded-full"></div>
            <div v-if="algo.disabled" class="absolute top-1 right-1 px-1.5 py-0.5 text-xs bg-gray-100 text-gray-500 rounded-full">开发中</div>
          </button>
        </div>
      </div>
    </div>

    <!-- 阈值设置 -->
    <div v-if="showThreshold" class="space-y-2">
      <div class="flex justify-between items-center">
        <div class="text-xs font-medium text-gray-500 uppercase tracking-wider">相似度阈值</div>
        <div class="text-sm font-medium text-blue-600">{{ similarityThreshold }}%</div>
      </div>
      <div class="relative">
        <input
          type="range"
          :value="similarityThreshold"
          @input="e => updateThreshold(Number(e.target.value))"
          min="0"
          max="100"
          step="1"
          class="w-full h-1.5 bg-gray-200 rounded-full appearance-none cursor-pointer"
        />
        <div class="absolute -bottom-4 left-0 w-full flex justify-between text-xs text-gray-400">
          <span>宽松</span>
          <span>严格</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 滑块样式 */
input[type="range"]::-webkit-slider-thumb {
  appearance: none;
  width: 12px;
  height: 12px;
  background-color: #3b82f6;
  border-radius: 50%;
  cursor: pointer;
  transition: all 0.2s;
}

input[type="range"]::-webkit-slider-thumb:hover {
  background-color: #2563eb;
  transform: scale(1.2);
}

input[type="range"]::-moz-range-thumb {
  width: 12px;
  height: 12px;
  background-color: #3b82f6;
  border: none;
  border-radius: 50%;
  cursor: pointer;
  transition: all 0.2s;
}

input[type="range"]::-moz-range-thumb:hover {
  background-color: #2563eb;
  transform: scale(1.2);
}

/* 禁用状态动画 */
button:disabled {
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 0.5; }
  50% { opacity: 0.3; }
}

/* 文本截断 */
.line-clamp-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style> 