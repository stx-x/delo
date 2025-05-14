<template>
  <router-view v-slot="{ Component }">
    <transition
      name="fade"
      mode="out-in"
      appear
    >
      <component :is="Component" />
    </transition>
  </router-view>
</template>

<style>
/* 全局过渡效果 */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

/* 全局字体和基础样式 */
body {
  margin: 0;
  padding: 0;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

/* 修复一些Safari的问题 */
button {
  -webkit-appearance: none;
  appearance: none;
}

/* 禁用iOS上的点击高亮 */
* {
  -webkit-tap-highlight-color: transparent;
}
</style>