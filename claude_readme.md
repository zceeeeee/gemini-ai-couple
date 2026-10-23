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
- **统一存储引擎 (SQLite-All-In-One)**：关系数据 + FTS5 全文检索引擎 + 向量存储紧凑集成，无需部署外部重型向量库，适合嵌入式与轻量自托管。
- **杂化检索算法 (RRF, Reciprocal Rank Fusion)**：结合语义向量余弦近似搜索与 FTS5 BM25 词法匹配，兼顾模糊语义与专有名词精准命中。
- **FSRS-4.5 认知动力学模型**：落地基于时间衰减幂律的记忆可提取度曲线 $R(t, S)$，在遗忘临界点 $(1-R)$ 唤醒时触发指数级稳定性 $S$ 跃升强化，并在 $R \approx 0.5$ 半遗忘区间引入倒 U 型惊喜共鸣增益（Surprise Bonus）。
- **睡眠巩固与非破坏性知识重构（Reconsolidation）**：在闲置/夜间触发 L0 对话碎片重放与提炼，升华为 L1 事件、L2 事实与 L3 叙事反思；当出现事实变更（如喜好转移）时，通过 `valid_to` 时间戳软注销旧事实，保留认知演变历史。
- **双模嵌入支持**：抽象 `EmbeddingEngine` trait，提供零依赖轻量 N-gram 特征哈希引擎（测试与离线环境）和标准化向量特征接口，平滑演进至本地 BGE ONNX 模型。
- **主动关怀非单纯定时器**：借鉴 Shikigami-Protocol，引入欲望值与内心独白评估机制（沉默时长、情感波动、关系阶段、用户作息），自然触发主动关怀。

## 3. 已知问题与现状
- **Phase 1 已落地**：Rust Workspace 骨架（`app_core`）、L0~L4 五层存储表及触发器、FTS5 BM25 检索、RRF 互惠排名多路召回融合均已完成并通过最小可验证端到端测试。
- **Phase 2 已落地**：FSRS-4.5 动力学衰减曲线、遗忘临界唤醒强化更新、沉睡记忆阈值归档、睡眠巩固（L0 提炼升华为 L1/L2/L3）及非破坏性矛盾修正（Reconsolidation）已全部实现并通过端到端测试；Android (ARM64) 与鸿蒙 (ARM64) 交叉编译检查通过。
- **环境要求**：开发构建与交叉编译需在激活的隔离环境（`.\activate.ps1`）下进行。
- **下一阶段**：Phase 3 的 ASE 主动关怀引擎（欲望值计算 + 内心独白自省机制）。

## 4. 架构约定
- **代码组织**：
  - `crates/app_core`: 纯业务核心（记忆管理、LLM 编排、主动引擎、模型协议、存储层）
    - `src/storage`: SQLite All-in-One 存储引擎 (L0~L4 模型、FTS5 触发器、CRUD、事实有效时间段管理)
    - `src/embedding`: 向量嵌入 Trait、数学工具库（余弦相似度、范数）与确定性特征引擎
    - `src/retrieval`: RRF 互惠排名算法与 HybridRetriever 双路召回融合 (支持动态 FSRS 生命力加权)
    - `src/dynamics`: FSRS 认知动力学模型 (时间衰减曲线、间隔重复强化、难度调整、惊喜增益与 MemoryDynamicsManager)
    - `src/consolidation`: 睡眠巩固流水线 (ConsolidationExtractor 提炼接口、规则提炼引擎与 ConsolidationWorker 升华工作流)
  - `crates/gateway`: 多通道适配器（OneBot QQ、WeChat、WebSocket、HTTP API）
  - `crates/bridge_android`: UniFFI 绑定层（向 Android 提供原生接口）
  - `crates/bridge_harmony`: ohos-rs 绑定层（向鸿蒙提供 Node-API 模块）
  - `apps/android_app`: Android 原生 Jetpack Compose 应用工程
- **命名规范**：
  - Rust: 类型 `UpperCamelCase`，函数与变量 `snake_case`，模块与文件名 `snake_case`
  - 数据库表: 复数小写下划线（如 `conversations`, `episodic_memories`, `semantic_facts`, `narratives`, `profiles`）
- **错误处理**：统一采用 `thiserror` 定义 `AppError`，暴露标准 `AppResult<T>`。

## 5. 待办事项与开发路线
> 完整分模块里程碑与参考对象请参见详细路线图：[docs/ROADMAP.md](docs/ROADMAP.md)

- [x] **Phase 1**: 搭建 Rust Workspace 骨架（`app_core`），实现 L0~L4 SQLite 存储架构与 FastEmbed+FTS5 RRF 混合检索
- [x] **Phase 2**: 实现 FSRS 记忆衰减模型与后台睡眠巩固守护进程（Sleep Consolidation）
- [ ] **Phase 3**: 实现 ASE 主动关怀引擎（欲望值计算 + 内心独白自省机制）
- [ ] **Phase 4**: 实现上下文合成器与 LLM 编排层，完成 `cli_preview` 终端交互闭环
- [ ] **Phase 5**: 实现 `crates/gateway`，接入 OneBot v11 (NapCat QQ) 与微信网关
- [ ] **Phase 6**: 搭建 Android Jetpack Compose 客户端与记忆透视窗（Memory Inspector）
