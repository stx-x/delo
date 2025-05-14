<script setup>
import { computed } from 'vue';

const props = defineProps({
  image: {
    type: Object,
    required: true
  },
  selected: {
    type: Boolean,
    default: false
  },
  index: {
    type: Number,
    default: 0
  },
  groupIndex: {
    type: Number,
    default: 0
  },
  isFirst: {
    type: Boolean,
    default: false
  }
});

const emit = defineEmits(['select', 'preview', 'open']);

// Format file size
const formattedSize = computed(() => {
  const bytes = props.image.size_bytes;
  if (!bytes) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(2)} ${units[i]}`;
});

// Format dimensions
const dimensions = computed(() => {
  return `${props.image.width} × ${props.image.height}`;
});

// Format filename
const filename = computed(() => {
  if (!props.image.path) return '';
  const parts = props.image.path.split(/[/\\]/);
  return parts[parts.length - 1];
});

// Calculate quality score (higher is better)
const qualityScore = computed(() => {
  const pixels = props.image.width * props.image.height;
  const size = props.image.size_bytes;
  if (!pixels || !size) return 0;
  
  // Simple quality metric: pixels per byte (higher is better)
  return pixels / size;
});

// Toggle selection
const toggleSelection = () => {
  emit('select', { groupIndex: props.groupIndex, imageIndex: props.index });
};

// Open image preview
const openPreview = () => {
  emit('preview', { image: props.image, groupIndex: props.groupIndex, imageIndex: props.index });
};

// Open image in native viewer
const openImage = (event) => {
  event.stopPropagation();
  emit('open', props.image.path);
};
</script>

<template>
  <div 
    class="rounded-lg overflow-hidden border transition-all duration-300 group"
    :class="[
      selected ? 'border-blue-500 bg-blue-50' : 'border-gray-200 bg-white hover:border-gray-300',
      isFirst ? 'ring-2 ring-blue-200 ring-offset-2' : ''
    ]"
  >
    <!-- Image container -->
    <div 
      class="relative h-48 overflow-hidden cursor-pointer"
      @click="openPreview"
    >
      <img 
        :src="image.dataUrl || window.__TAURI__.core.convertFileSrc(image.path)"
        :alt="filename"
        class="w-full h-full object-cover transition-all group-hover:scale-105"
        loading="lazy"
      />
      
      <!-- Overlay with actions -->
      <div class="absolute inset-0 bg-black/50 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center gap-2">
        <button 
          @click.stop="openImage"
          class="bg-white/90 p-2 rounded-full hover:bg-white transition-colors"
          title="在系统中打开"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path>
            <polyline points="15 3 21 3 21 9"></polyline>
            <line x1="10" y1="14" x2="21" y2="3"></line>
          </svg>
        </button>
        
        <button 
          @click.stop="openPreview"
          class="bg-white/90 p-2 rounded-full hover:bg-white transition-colors"
          title="预览图片"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8"></circle>
            <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
          </svg>
        </button>
      </div>
      
      <!-- Quality indicator -->
      <div 
        class="absolute top-2 right-2 py-1 px-1.5 rounded text-xs font-medium"
        :class="isFirst ? 'bg-blue-500 text-white' : 'bg-white/80 text-gray-800'"
        v-if="isFirst"
      >
        推荐保留
      </div>
    </div>
    
    <!-- Image info -->
    <div class="p-3">
      <div class="flex justify-between items-start mb-2">
        <div class="truncate text-sm font-medium text-gray-800" :title="filename">
          {{ filename }}
        </div>
        <div class="flex-shrink-0">
          <input 
            type="checkbox" 
            :checked="selected" 
            @change="toggleSelection"
            class="w-4 h-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500"
          />
        </div>
      </div>
      
      <!-- Image metadata -->
      <div class="text-xs text-gray-500 space-y-1">
        <div class="flex items-center gap-1">
          <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
            <circle cx="8.5" cy="8.5" r="1.5"></circle>
            <polyline points="21 15 16 10 5 21"></polyline>
          </svg>
          <span>{{ dimensions }}</span>
        </div>
        <div class="flex items-center gap-1">
          <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
            <polyline points="17 8 12 3 7 8"></polyline>
            <line x1="12" y1="3" x2="12" y2="15"></line>
          </svg>
          <span>{{ formattedSize }}</span>
        </div>
        <div class="flex items-center gap-1">
          <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="10"></circle>
            <polyline points="12 6 12 12 16 14"></polyline>
          </svg>
          <span>{{ image.formattedDate }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.truncate {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

input[type="checkbox"] {
  cursor: pointer;
  border-radius: 0.25rem;
}

input[type="checkbox"]:checked {
  background-color: #3b82f6;
  border-color: #3b82f6;
}
</style>