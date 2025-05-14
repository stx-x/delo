<script setup>
import { ref, computed, watch, nextTick } from 'vue';
import Button from './ui/Button.vue';

const props = defineProps({
  show: {
    type: Boolean,
    default: false
  },
  image: {
    type: Object,
    default: null
  },
  groupIndex: {
    type: Number,
    default: 0
  },
  imageIndex: {
    type: Number,
    default: 0
  },
  totalImages: {
    type: Number,
    default: 0
  },
  selected: {
    type: Boolean,
    default: false
  }
});

const emit = defineEmits([
  'close', 
  'prev', 
  'next', 
  'toggle-selection', 
  'delete', 
  'open'
]);

const loading = ref(true);
const fullscreen = ref(false);
const zoom = ref(1);
const imageElement = ref(null);
const previewContainer = ref(null);

// Reset zoom when image changes
watch(() => props.image, () => {
  zoom.value = 1;
  loading.value = true;
});

// Format file size
const formattedSize = computed(() => {
  if (!props.image) return '0 B';
  const bytes = props.image.size_bytes;
  if (!bytes) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(2)} ${units[i]}`;
});

// Handle keyboard navigation
const handleKeydown = (e) => {
  if (!props.show) return;
  
  switch(e.key) {
    case 'ArrowLeft':
      emit('prev');
      break;
    case 'ArrowRight':
      emit('next');
      break;
    case 'Escape':
      fullscreen.value ? toggleFullscreen() : emit('close');
      break;
    case ' ':
      emit('toggle-selection');
      break;
    case 'f':
      toggleFullscreen();
      break;
    case '+':
    case '=':
      zoomIn();
      break;
    case '-':
      zoomOut();
      break;
    case '0':
      zoom.value = 1;
      break;
  }
};

// Toggle fullscreen mode
const toggleFullscreen = async () => {
  fullscreen.value = !fullscreen.value;
  
  if (fullscreen.value) {
    try {
      if (previewContainer.value.requestFullscreen) {
        await previewContainer.value.requestFullscreen();
      } else if (previewContainer.value.webkitRequestFullscreen) {
        await previewContainer.value.webkitRequestFullscreen();
      }
    } catch (err) {
      console.error('Fullscreen failed:', err);
      fullscreen.value = false;
    }
  } else {
    try {
      if (document.exitFullscreen) {
        await document.exitFullscreen();
      } else if (document.webkitExitFullscreen) {
        await document.webkitExitFullscreen();
      }
    } catch (err) {
      console.error('Exit fullscreen failed:', err);
    }
  }
};

// Zoom controls
const zoomIn = () => {
  zoom.value = Math.min(zoom.value + 0.25, 3);
};

const zoomOut = () => {
  zoom.value = Math.max(zoom.value - 0.25, 0.5);
};

const resetZoom = () => {
  zoom.value = 1;
};

// Handle image load event
const onImageLoad = () => {
  loading.value = false;
};

// Open image in system viewer
const openInSystem = () => {
  emit('open', props.image?.path);
};

// Drag to scroll when zoomed
let isDragging = false;
let dragStartX = 0;
let dragStartY = 0;
let scrollLeft = 0;
let scrollTop = 0;

const startDrag = (e) => {
  if (zoom.value <= 1) return;
  isDragging = true;
  dragStartX = e.clientX;
  dragStartY = e.clientY;
  scrollLeft = previewContainer.value.scrollLeft;
  scrollTop = previewContainer.value.scrollTop;
  previewContainer.value.style.cursor = 'grabbing';
};

const drag = (e) => {
  if (!isDragging) return;
  const dx = e.clientX - dragStartX;
  const dy = e.clientY - dragStartY;
  previewContainer.value.scrollLeft = scrollLeft - dx;
  previewContainer.value.scrollTop = scrollTop - dy;
};

const endDrag = () => {
  isDragging = false;
  previewContainer.value.style.cursor = zoom.value > 1 ? 'grab' : 'default';
};

// Add event listeners
watch(() => props.show, (value) => {
  if (value) {
    nextTick(() => {
      window.addEventListener('keydown', handleKeydown);
    });
  } else {
    window.removeEventListener('keydown', handleKeydown);
  }
});

// Clean up event listeners
onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown);
});
</script>

<template>
  <div 
    v-if="show" 
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-sm transition-opacity"
    @click="emit('close')"
  >
    <!-- Preview container -->
    <div 
      ref="previewContainer"
      class="relative w-full h-full overflow-auto flex flex-col"
      @click.stop
      @mousedown="startDrag"
      @mousemove="drag"
      @mouseup="endDrag"
      @mouseleave="endDrag"
      :class="{ 'cursor-grab': zoom > 1 }"
    >
      <!-- Toolbar -->
      <div class="flex justify-between items-center p-4 z-10">
        <!-- Left controls -->
        <div class="flex items-center gap-3">
          <Button 
            variant="ghost" 
            size="sm"
            @click="emit('close')"
            class="text-white"
          >
            <template #icon-left>
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <line x1="18" y1="6" x2="6" y2="18"></line>
                <line x1="6" y1="6" x2="18" y2="18"></line>
              </svg>
            </template>
            关闭
          </Button>
          
          <span class="text-white/90 text-sm">
            {{ imageIndex + 1 }} / {{ totalImages }}
          </span>
        </div>
        
        <!-- Right controls -->
        <div class="flex items-center gap-2">
          <Button 
            variant="ghost" 
            size="sm" 
            @click="emit('toggle-selection')"
            class="text-white"
          >
            <template #icon-left>
              <svg v-if="selected" xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="9 11 12 14 22 4"></polyline>
                <path d="M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11"></path>
              </svg>
              <svg v-else xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
              </svg>
            </template>
            {{ selected ? '已选择' : '选择' }}
          </Button>
          
          <Button 
            variant="ghost" 
            size="sm"
            @click="openInSystem"
            class="text-white"
          >
            <template #icon-left>
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path>
                <polyline points="15 3 21 3 21 9"></polyline>
                <line x1="10" y1="14" x2="21" y2="3"></line>
              </svg>
            </template>
            打开
          </Button>
          
          <Button 
            variant="danger" 
            size="sm"
            @click="emit('delete')"
            class="bg-red-500 hover:bg-red-600"
          >
            <template #icon-left>
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M3 6h18"></path>
                <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"></path>
                <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
              </svg>
            </template>
            删除
          </Button>
        </div>
      </div>
      
      <!-- Image container -->
      <div class="flex-grow flex items-center justify-center">
        <!-- Loading indicator -->
        <div v-if="loading" class="absolute inset-0 flex items-center justify-center">
          <div class="w-12 h-12 border-4 border-blue-500 border-t-transparent rounded-full animate-spin"></div>
        </div>
        
        <!-- Image -->
        <img
          v-if="image && image.path"
          ref="imageElement"
          :src="image.dataUrl || window.__TAURI__.core.convertFileSrc(image.path)"
          :alt="image.path.split('/').pop()"
          class="transition-transform"
          :style="{ transform: `scale(${zoom})` }"
          @load="onImageLoad"
        />
      </div>
      
      <!-- Bottom controls -->
      <div class="flex justify-between items-center p-4 bg-black/40 backdrop-blur-sm">
        <!-- Image info -->
        <div class="text-white space-y-1">
          <div class="font-medium">{{ image?.path?.split('/').pop() }}</div>
          <div class="text-sm text-white/80">
            {{ image?.width }} × {{ image?.height }} · {{ formattedSize }} · {{ image?.formattedDate }}
          </div>
        </div>
        
        <!-- Controls -->
        <div class="flex items-center gap-2">
          <!-- Zoom controls -->
          <div class="flex items-center bg-black/20 rounded-full p-1">
            <button 
              @click="zoomOut"
              class="w-8 h-8 flex items-center justify-center text-white rounded-full hover:bg-black/20"
              :disabled="zoom <= 0.5"
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <line x1="5" y1="12" x2="19" y2="12"></line>
              </svg>
            </button>
            
            <button
              @click="resetZoom"
              class="px-2 text-white text-sm hover:underline"
            >
              {{ Math.round(zoom * 100) }}%
            </button>
            
            <button 
              @click="zoomIn"
              class="w-8 h-8 flex items-center justify-center text-white rounded-full hover:bg-black/20"
              :disabled="zoom >= 3"
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <line x1="12" y1="5" x2="12" y2="19"></line>
                <line x1="5" y1="12" x2="19" y2="12"></line>
              </svg>
            </button>
          </div>
          
          <!-- Fullscreen toggle -->
          <button 
            @click="toggleFullscreen"
            class="w-10 h-10 flex items-center justify-center text-white rounded-full hover:bg-black/20"
          >
            <svg v-if="!fullscreen" xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M8 3H5a2 2 0 0 0-2 2v3"></path>
              <path d="M21 8V5a2 2 0 0 0-2-2h-3"></path>
              <path d="M3 16v3a2 2 0 0 0 2 2h3"></path>
              <path d="M16 21h3a2 2 0 0 0 2-2v-3"></path>
            </svg>
            <svg v-else xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M8 3v3a2 2 0 0 1-2 2H3"></path>
              <path d="M21 8h-3a2 2 0 0 1-2-2V3"></path>
              <path d="M3 16h3a2 2 0 0 1 2 2v3"></path>
              <path d="M16 21v-3a2 2 0 0 1 2-2h3"></path>
            </svg>
          </button>
          
          <!-- Navigation controls -->
          <div class="flex items-center bg-black/20 rounded-full">
            <button 
              @click="emit('prev')"
              class="w-10 h-10 flex items-center justify-center text-white rounded-full hover:bg-black/20"
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="m15 18-6-6 6-6"/>
              </svg>
            </button>
            
            <button 
              @click="emit('next')"
              class="w-10 h-10 flex items-center justify-center text-white rounded-full hover:bg-black/20"
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="m9 18 6-6-6-6"/>
              </svg>
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Transition effects */
.transition-opacity {
  transition: opacity 0.2s ease;
}

/* Animation for loading spinner */
@keyframes spin {
  to { transform: rotate(360deg); }
}
.animate-spin {
  animation: spin 1s linear infinite;
}

/* Scroll customization when zoomed */
.overflow-auto {
  scrollbar-width: thin;
  scrollbar-color: rgba(255, 255, 255, 0.3) transparent;
}

.overflow-auto::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

.overflow-auto::-webkit-scrollbar-track {
  background: transparent;
}

.overflow-auto::-webkit-scrollbar-thumb {
  background-color: rgba(255, 255, 255, 0.3);
  border-radius: 4px;
}

.overflow-auto::-webkit-scrollbar-thumb:hover {
  background-color: rgba(255, 255, 255, 0.5);
}
</style>