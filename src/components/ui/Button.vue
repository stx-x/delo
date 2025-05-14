<template>
  <button
    :type="type"
    :class="[
      'inline-flex items-center justify-center gap-2 transition-all duration-300 font-medium focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500',
      sizeClasses,
      variantClasses,
      radiusClasses,
      {
        'opacity-60 pointer-events-none cursor-not-allowed': disabled,
        'animate-pulse': loading,
      },
      className
    ]"
    :disabled="disabled || loading"
    @click="$emit('click', $event)"
  >
    <!-- 加载状态图标 -->
    <span v-if="loading" class="animate-spin">
      <svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M21 12a9 9 0 1 1-6.219-8.56" />
      </svg>
    </span>
    
    <!-- 前置图标 -->
    <slot name="icon-left"></slot>
    
    <!-- 按钮内容 -->
    <span><slot></slot></span>
    
    <!-- 后置图标 -->
    <slot name="icon-right"></slot>
  </button>
</template>

<script setup>
import { computed } from 'vue';

const props = defineProps({
  // 按钮类型
  type: {
    type: String,
    default: 'button'
  },
  // 尺寸: 'xs', 'sm', 'md', 'lg', 'xl'
  size: {
    type: String,
    default: 'md'
  },
  // 样式变体: 'primary', 'secondary', 'outline', 'ghost', 'link', 'danger'
  variant: {
    type: String,
    default: 'primary'
  },
  // 圆角: 'none', 'sm', 'md', 'lg', 'full'
  radius: {
    type: String,
    default: 'md'
  },
  // 禁用状态
  disabled: {
    type: Boolean,
    default: false
  },
  // 加载状态
  loading: {
    type: Boolean,
    default: false
  },
  // 自定义类名
  className: {
    type: String,
    default: ''
  }
});

defineEmits(['click']);

// 尺寸类映射
const sizeClasses = computed(() => {
  const sizes = {
    'xs': 'px-2 py-1 text-xs',
    'sm': 'px-3 py-1.5 text-sm',
    'md': 'px-4 py-2 text-base',
    'lg': 'px-6 py-3 text-lg',
    'xl': 'px-8 py-4 text-xl'
  };
  
  return sizes[props.size] || sizes.md;
});

// 变体类映射
const variantClasses = computed(() => {
  const variants = {
    'primary': 'bg-blue-500 text-white hover:bg-blue-600 hover:-translate-y-1 hover:shadow-lg active:translate-y-0 active:shadow-md',
    'secondary': 'bg-gray-200 text-gray-800 hover:bg-gray-300 hover:-translate-y-0.5',
    'outline': 'bg-transparent border border-current text-blue-500 hover:bg-blue-50',
    'ghost': 'bg-transparent hover:bg-gray-100 text-gray-700',
    'link': 'bg-transparent p-0 text-blue-500 hover:underline shadow-none',
    'danger': 'bg-red-500 text-white hover:bg-red-600 hover:-translate-y-1 hover:shadow-lg'
  };
  
  return variants[props.variant] || variants.primary;
});

// 圆角类映射
const radiusClasses = computed(() => {
  const radiuses = {
    'none': 'rounded-none',
    'sm': 'rounded-sm',
    'md': 'rounded-md',
    'lg': 'rounded-lg',
    'xl': 'rounded-xl',
    '2xl': 'rounded-2xl',
    'full': 'rounded-full'
  };
  
  return radiuses[props.radius] || radiuses.md;
});
</script>