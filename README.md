# Guitar Sheet 谱酷

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

## GitHub 同步（可选）

在应用 **设置** 中配置仓库（`owner/repo`）、分支、可选子目录与 **Personal Access Token（PAT）**。Token 保存在系统安全存储（如 macOS 钥匙串），不会写入谱子文件或可被误提交的明文配置。

### PAT 权限（Classic token）

- **私有仓库**：勾选 **`repo`**（完整仓库访问），否则无法读写 Contents API。  
- **仅公开仓库**：一般仍需能写文件；若使用 fine-grained token，请为该仓库授予 **Contents：Read and write**。具体以 GitHub 文档为准，原则是**最小够用权限**。  
- 在 GitHub：**Settings → Developer settings → Personal access tokens** 创建或轮换 token。

### 单文件大小与 API 限制

- GitHub **通过 REST Contents API 更新的单个文件**有大小上限（常见文档为约 **100 MB**）。超过上限的 PDF/扫描件会同步失败，应用会提示错误；本地阅读不受影响。  
- 若遇 **rate limit**，请稍后再试或检查是否在短时间内大量同步。

### Token 轮换建议

1. 在 GitHub 生成新 PAT，复制备用。  
2. 打开本应用 **设置**，粘贴新 token 并保存（会覆盖旧值）。  
3. 在 GitHub 上**撤销旧 PAT**（若仍保留旧 token，两处并存期间以最后写入应用的为准）。

### 冲突处理（策略 B）

当本地与远端相对「上次已知基线」都已变更时，同步会**中止并列出冲突**。请在每条目上选择：**保留本地**（以本地覆盖远端）、**使用远端**（覆盖本地副本）、或**另存为新文件**（避免静默覆盖）。

## 手动测试清单（MVP）

与产品规格中的手动验证项对齐，发版或改动同步/导入路径后建议过一遍：

- [ ] **导入**：分别导入文本 TAB（`.txt` / `.md`）、PDF、图片，谱库中出现条目且可打开阅读。  
- [ ] **离线**：断网后仍可打开已导入谱子。  
- [ ] **同步无冲突**：配置有效 PAT 与测试仓库，修改一条谱子后执行同步，远端与本地一致。  
- [ ] **冲突三操作**：人为制造「本地与远端都已改」的场景，依次验证 **保留本地 / 使用远端 / 另存为新文件** 后索引与文件内容符合预期。  
- [ ] **Token 错误**：故意填入无效或过期 token，应出现可理解错误（而非静默失败）。  
- [ ] **大文件**：选用接近或超过 GitHub 单文件限制的文件尝试同步，应得到清晰失败提示。

## 推荐开发环境

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
