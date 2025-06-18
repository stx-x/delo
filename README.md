# Delo - 开发与打包

## 项目简介

Delo 是一个基于 Tauri + Vue3 开发的桌面应用程序。

## 环境要求

- **Node.js**: 22.x LTS 版本
- **Rust**: 最新稳定版本
- **操作系统**: Windows 10+、macOS 10.15+、Linux

## 环境安装

### 1. 安装 Node.js

访问 [Node.js 官网](https://nodejs.org/zh-cn/download) 下载并安装 22.x LTS 版本。

安装完成后，在命令行中验证：

```bash
node --version
```

**注意事项**：如果安装后命令行提示"找不到命令"，需要重启终端或配置环境变量。Windows用户可能需要将Node.js安装路径添加到系统PATH环境变量中。

### 2. 安装 Rust

访问 [Rust 官网](https://www.rust-lang.org/zh-CN/tools/install) 按照指导安装。

类 unix 平台可直接执行：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

安装完成后验证：

```bash
rustc --version
```

**环境变量配置**：

- 如果安装后命令行提示"找不到命令"，需要重启终端使环境变量生效
- Windows用户如果遇到问题，需要手动添加Rust工具链路径到PATH环境变量：
  - 通常路径为：`C:\Users\你的用户名\.cargo\bin`
- macOS/Linux用户通常会自动配置，如有问题可执行：`source ~/.bashrc` 或 `source ~/.zshrc`

## 项目配置

### 1. 安装项目依赖

在项目根目录执行：

```bash
npm install
```

### 2. 安装 Tauri 命令行工具

```bash
cargo install tauri-cli --version "^2.0.0" --locked
```

## 运行项目

### 开发模式

```bash
cargo tauri dev
```

*注：首次运行需要编译，耗时较长（约10-20分钟），请耐心等待。*

### 打包发布

```bash
cargo tauri build
```

生成的安装包位于 `src-tauri/target/release/bundle/` 目录下。

## 常见问题

### 依赖安装失败

- 检查 Node.js 和 Rust 是否正确安装
- 网络问题可尝试使用国内镜像源

### 编译时间过长

- 首次编译需要下载和编译大量依赖，属于正常现象
- 后续编译速度会显著提升

### 运行错误

- 确保在项目**根目录**下执行命令
- 检查所有依赖是否正确安装

## 参考文档

- [Tauri 命令行工具参考](https://v2.tauri.app/zh-cn/reference/cli/)
- [NPM 包管理器介绍]([https://nodejs.org/zh-cn/learn/getting-started/an-introduction-to-the-npm-package-manager#an-introduction-to-the-npm-package-manager)
- [The Cargo Book](https://doc.rust-lang.org/cargo/)
- [Node.js 官方安装指南](https://nodejs.org/zh-cn/download)
- [Rust 官方安装指南](https://www.rust-lang.org/zh-CN/tools/install)
- [Tauri 官方文档](https://v2.tauri.app/zh-cn/)
- [Node.js 官方文档](https://nodejs.org/zh-cn/)
- [Rust 官方文档](https://www.rust-lang.org/zh-CN/)
- [The Rust Book](https://doc.rust-lang.org/book/)
- [The Rust Book 非官方中文版](https://kaisery.github.io/trpl-zh-cn/)
