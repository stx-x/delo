<template>
  <div class="min-h-screen bg-slate-50 flex flex-col">
    <!-- 应用标题栏 -->
    <header class="w-full px-6 py-4 bg-white shadow-sm z-10">
      <div class="max-w-7xl mx-auto flex items-center justify-between">
        <!-- 品牌标识 -->
        <div class="flex items-center gap-2">
          <router-link to="/" class="flex items-center gap-2 hover:opacity-80 transition-opacity">
            <h1 class="text-2xl font-black text-gray-900 tracking-tight flex items-center gap-2">
              <span class="bg-gradient-to-r from-blue-500 to-blue-600 bg-clip-text text-transparent font-['SF_Pro_Display',-apple-system,BlinkMacSystemFont,'Segoe_UI',Roboto,sans-serif] relative animate-gradient-flow hover:animate-gradient-flow-running">Delo</span>
              <span class="text-slate-400 font-light transition-transform duration-300">·</span>
              <span class="text-gray-900 font-bold">去若</span>
            </h1>
          </router-link>
        </div>

        <!-- 导航菜单 -->
        <nav class="hidden md:flex items-center space-x-6">
          <router-link 
            to="/" 
            class="text-slate-600 hover:text-blue-500 transition-colors px-2 py-1 text-sm font-medium"
            active-class="text-blue-500 font-semibold"
            exact
          >
            首页
          </router-link>
          <a 
            href="https://github.com/user/delo" 
            target="_blank" 
            rel="noopener noreferrer"
            class="text-slate-600 hover:text-blue-500 transition-colors px-2 py-1 text-sm font-medium flex items-center gap-1"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22"></path>
            </svg>
            GitHub
          </a>
        </nav>

        <!-- 移动端菜单按钮 -->
        <button class="md:hidden p-2 rounded-md text-slate-600 hover:text-blue-500 hover:bg-slate-100 transition-colors">
          <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="3" y1="12" x2="21" y2="12"></line>
            <line x1="3" y1="6" x2="21" y2="6"></line>
            <line x1="3" y1="18" x2="21" y2="18"></line>
          </svg>
        </button>
      </div>
    </header>

    <!-- 主内容区域 -->
    <main class="flex-1 w-full">
      <slot></slot>
    </main>

    <!-- 页脚 -->
    <footer class="w-full py-6 px-6 bg-white border-t border-slate-200">
      <div class="max-w-7xl mx-auto text-center text-slate-500 text-sm">
        <p>© {{ new Date().getFullYear() }} Delo - 去若 | 专业的图片重复检测工具</p>
      </div>
    </footer>
  </div>
</template>

<script setup>
// 这里是 AppLayout 组件的逻辑
</script>

<style scoped>
/* 布局特定样式 */
.animate-gradient-flow {
  animation: gradient-flow 2.5s linear infinite paused;
  background-size: 200% auto;
}

.animate-gradient-flow-running {
  animation-play-state: running;
}

@keyframes gradient-flow {
  0% { background-position: 0% 50%; }
  50% { background-position: 100% 50%; }
  100% { background-position: 0% 50%; }
}
</style>