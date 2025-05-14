import { createApp } from "vue";
import { createRouter, createWebHashHistory } from "vue-router";
import App from "./App.vue";
import Home from "./views/Home.vue";
import Results from "./views/Results.vue";

// 导入全局样式
import "./assets/css/main.css";

// 创建全局状态存储
const globalState = {
  duplicateGroups: [],
  selectedFolders: [],
  algorithm: '',
  similarityThreshold: 0,
  recursive: true
};

// 创建路由
const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: "/",
      name: "home",
      component: Home,
      meta: {
        title: '首页 - Delo 去若'
      }
    },
    {
      path: "/results/:timestamp",
      name: "results",
      component: Results,
      props: true,
      meta: {
        title: '处理结果 - Delo 去若'
      }
    }
  ]
});

// 路由标题处理
router.beforeEach((to, from, next) => {
  document.title = to.meta.title || 'Delo 去若 - 重复图片处理工具';
  next();
});

const app = createApp(App);
app.use(router);

// 将全局状态注入到应用中
app.provide('globalState', globalState);

app.mount("#app");