# 开发说明

## 技术栈

- Tauri 2
- Rust
- Vue 3
- TypeScript
- Ant Design Vue

## 常用命令

```bash
cd desktop-client
npm install
npm run dev
npm run build
```

```bash
cd tauri-desktop
npm install
npm run tauri:dev
npm run tauri:build
```

`tauri:dev` 会自动启动 `desktop-client` 的 Vite 服务。

Rust 适配器验证：

```bash
cd tauri-desktop/src-tauri
cargo check
cargo test
cargo test smoke_search_video_keywords -- --ignored --nocapture
```

## 分层约定

- `desktop-client/src/views/SearchWorkbench.vue`：页面状态、交互和结果展示。
- `desktop-client/src/api/native.ts`：Tauri 命令类型和调用封装。
- `tauri-desktop/src-tauri/src/lib.rs`：搜索来源、规则适配、详情解析、CMS 健康检测和外部打开命令。
- `rules/sources/*.json`：本地页面源元数据，当前包括混合盘、皮卡搜索、阿里搜、盘趣多等公开页面源，以及千帆、易搜等需配置源。

## 搜索准确性链路

- 影视实体识别：`MediaCandidate` 根据用户输入识别片名、年份、类型、主演、别名和平台。
- 搜索计划：`SearchPlan` 将自然语言输入拆成多个资源搜索词。
- 资源召回：PanSou 深度池、CMS V10 源池、Torznab/Newznab 外部索引器、本地页面源并发执行，单源失败不影响其他来源。
- 来源覆盖：搜索响应返回 `coverage`，前端按资源池类型显示返回量、失败量和禁用量。
- 结果重排：按片名、年份、主演、平台、影视信号、噪声词和链接有效性计算评分，并分为高可信、可能相关和低相关。
- 目标命中：搜索响应返回 `targetResourceCount` 和 `targetResourceMessage`，`生命树` 这类目标剧只把“片名命中 + 影视信号”的网盘结果计入成功。

本地登录只校验客户端用户文件；前端使用 `localStorage` 缓存用户名、显示名和登录时间，不保存密码、Token 或有效期。缓存会在用户主动退出登录或清除应用数据时删除；首版不引入服务端账号系统、数据库、服务端或移动端业务代码。
