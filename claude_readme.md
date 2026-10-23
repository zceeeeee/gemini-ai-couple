# 项目说明书 (claude_readme.md)

## 1. 项目概述
- **项目类型**：带多层认知记忆、自我遗忘与巩固、全渠道 IM 接入（QQ/微信/Android 端）的自托管 AI 伴侣系统。
- **技术栈**：
  - **核心后端**：Rust (Tokio, Axum, Sqlite/rusqlite, FastEmbed / ONNX embedding, FTS5)
  - **移动客户端**：Android 原生 (Kotlin + Jetpack Compose)，支持 UniFFI 原生库内嵌或 WebSocket/HTTP 远程连接双模式
  - **鸿蒙适配**：支持通过 `ohos-rs` (Node-API) 桥接至 HarmonyOS NEXT
  - **外部接入**：OneBot v11 (NapCat / Lagrange) 接入 QQ，WeChat Hook / Gateway 接入微信
- **主要功能**：
  - L0~L4 分层认知记忆体系（工作记忆、情景事件、语义事实、长程叙事、核心特质）
  - 基于 FSRS / 艾宾浩斯曲线的动态遗忘、记忆巩固与重放沉淀
  - ASE (Active Spontaneous Expression) 自主潜意识驱动的内心独白与主动关心
  - 多渠道消息统一路由与多会话身份映射 (Unified Identity Hub)

## 2. 关键决策
- **Shared Core 共享内核**：算法、记忆逻辑、数据库、协议均在 Rust 中实现，Android/鸿蒙等客户端仅作为展示层，保证多端逻辑 100% 统一且零重复逻辑。
- **纯 Rust 依赖优先**：避免 OpenSSL / 复杂 C++ 传递依赖，优先使用 `rustls`, `bundled` SQLite，确保交叉编译顺利。
- **统一存储引擎 (SQLite-All-In-One)**：关系数据 + FTS5 全文检索引擎 + 向量存储紧凑集成，无需部署庞大的外部向量数据库（如 Milvus / Qdrant），适合嵌入式与轻量自托管。
- **杂化检索算法 (RRF, Reciprocal Rank Fusion)**：结合语义向量近似搜索与 BM25 词法匹配，兼顾模糊语义与专有名词精准命中。
- **主动关怀非单纯定时器**：借鉴 Shikigami-Protocol，引入欲望值与内心独白评估机制（沉默时长、情感波动、关系阶段、用户作息），自然触发主动关怀。

## 3. 已知问题
- 当前仓库处于初始基础设施阶段（环境 `env/` 与交叉编译脚本已就绪，上层业务 Crates 准备起步搭建）。
- Android 与微信/QQ 网关的长期后台常驻及网络保活需针对各厂商系统配置电池白名单。

## 4. 架构约定
- **代码组织**：
  - `crates/app_core`: 纯业务核心（记忆管理、LLM 编排、主动引擎、模型协议）
  - `crates/gateway`: 多通道适配器（OneBot QQ、WeChat、WebSocket、HTTP API）
  - `crates/bridge_android`: UniFFI 绑定层（向 Android 提供原生接口）
  - `crates/bridge_harmony`: ohos-rs 绑定层（向鸿蒙提供 Node-API 模块）
  - `apps/android_app`: Android 原生 Jetpack Compose 应用工程
- **命名规范**：
  - Rust: 类型 `UpperCamelCase`，函数与变量 `snake_case`，模块与文件名 `snake_case`
  - 数据库表: 复数小写下划线（如 `memories`, `dialog_sessions`）
- **错误处理**：统一采用 `thiserror` 定义模块级强类型错误，核心上抛通过 `anyhow` 或统一 `AppError`。

## 5. 待办事项与开发路线
> 完整分模块里程碑与参考对象请参见详细路线图：[docs/ROADMAP.md](docs/ROADMAP.md)

- [ ] **Phase 1**: 搭建 Rust Workspace 骨架（`app_core`），实现 L0~L4 SQLite 存储架构与 FastEmbed+FTS5 RRF 混合检索
- [ ] **Phase 2**: 实现 FSRS 记忆衰减模型与后台睡眠巩固守护进程（Sleep Consolidation）
- [ ] **Phase 3**: 实现 ASE 主动关怀引擎（欲望值计算 + 内心独白自省机制）
- [ ] **Phase 4**: 实现上下文合成器与 LLM 编排层，完成 `cli_preview` 终端交互闭环
- [ ] **Phase 5**: 实现 `crates/gateway`，接入 OneBot v11 (NapCat QQ) 与微信网关
- [ ] **Phase 6**: 搭建 Android Jetpack Compose 客户端与记忆透视窗（Memory Inspector）
