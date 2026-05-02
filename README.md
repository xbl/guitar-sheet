# Guitar Sheet（吉他谱）

这是一款**面向初学者**的吉他谱桌面应用，用熟悉的界面帮你入门看谱、练谱。技术栈为 **Tauri 2 + Vue 3 + TypeScript**。

## 开发

在项目根目录安装依赖后启动桌面调试（含前端热更新）：

```bash
npm install
npm run tauri dev
```

仅启动前端（浏览器里调试 UI，不含 Tauri 原生能力）：

```bash
npm run dev
```

## 构建

打包生成 macOS 应用（会先执行类型检查与前端构建，再编译 Rust 并打 bundle）：

```bash
npm run tauri build
```

产物通常在 `src-tauri/target/release/bundle/macos/`（如 `.app` 等，具体以本机构建输出为准）。

## 推荐开发环境

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
