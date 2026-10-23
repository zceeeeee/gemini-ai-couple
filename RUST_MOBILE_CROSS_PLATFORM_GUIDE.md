# Rust 跨平台移动端开发全景实战指南（Android & HarmonyOS NEXT）

> 本文整理自知乎核心经验分享《用Rust实现跨平台开发(iOS/Android/Web)经验分享》与《Rust 与 Android 桥接：高性能应用开发》，并深度结合了业界（Mozilla Firefox、字节跳动、OpenHarmony 社区 `ohos-rs` 等）的最新生产级实战经验。

---

## 目录
1. [架构选型：为什么大厂普遍拥抱“Shared Core”模式？](#一架构选型为什么大厂普遍拥抱shared-core模式)
2. [两篇核心知乎文章深度复盘与精髓提炼](#二两篇核心知乎文章深度复盘与精髓提炼)
3. [Android 端实战方案：UniFFI 现代化桥接](#三android-端实战方案uniffi-现代化桥接)
4. [HarmonyOS NEXT（纯血鸿蒙）实战方案：ohos-rs (Node-API)](#四harmonyos-next纯血鸿蒙实战方案ohos-rs-node-api)
5. [全网生产级“踩坑”与避坑指南（关键干货）](#五全网生产级踩坑与避坑指南关键干货)
6. [推荐项目结构与工程骨架](#六推荐项目结构与工程骨架)

---

## 一、架构选型：为什么大厂普遍拥抱“Shared Core”模式？

在移动端使用 Rust，技术路线主要分为两派：

```mermaid
graph TD
    subgraph 路线 A: 纯 Rust UI 方案
        A1[Rust 业务逻辑] --> A2[Rust 渲染引擎 egui / Slint / Winit]
        A2 --> A3[直接画到 Native Canvas / Surface]
    end

    subgraph 路线 B: 共享核心 Shared Core (主流成熟)
        B1[Rust 共享核心层: 算法/数据/网络/协议] --> B2[桥接层: UniFFI / Node-API]
        B2 --> B3[Android UI: Kotlin + Jetpack Compose]
        B2 --> B4[鸿蒙 NEXT UI: ArkTS + ArkUI]
    end
```

### 两种路线对比：

| 评估维度 | 路线 A：纯 Rust UI (如 Slint, egui) | 路线 B：Shared Core (Rust核心 + 原生轻量UI) |
| :--- | :--- | :--- |
| **PC 端桌面预览** | ⭐️⭐️⭐️⭐️⭐️ (一条 `cargo run` 秒级启动预览) | ⭐️⭐️⭐️ (需单开轻量 CLI 或 Web/Desktop 壳测试逻辑) |
| **移动端原生体验** | ⭐️⭐️ (软键盘弹起、中文输入法避让、系统手势、无障碍支持易遇边界 Bug) | ⭐️⭐️⭐️⭐️⭐️ (100% 官方原生流畅度与设计语言) |
| **多端复用率** | 90% ~ 95% | 75% ~ 85% (所有非 UI 代码 100% 复用) |
| **系统生态接入** | 困难 (调用原生相机、生物识别、推送通知需频繁穿透 FFI) | 极易 (原生 UI 层直接调用系统 SDK，底层调用 Rust 处理数据) |
| **生产级适用度** | 适合工具类、工业面板、图形游戏、小型独立 App | 适合中大型 App、长线商业产品、大厂标准化团队 |

> [!TIP]
> **最佳实践决策**：
> * **前期开发与核心算法验证**：直接在 PC 端写纯 Rust 逻辑与命令行单元测试，效率最高。
> * **正式产品架构**：采用 **Shared Core 模式**。把加解密、离线数据库（SQLite/Redb）、数据同步、协议解析、复杂状态机都沉淀在 Rust 中，双端只用极简的原生代码渲染界面。

---

## 二、两篇核心知乎文章深度复盘与精髓提炼

### 1. 《用Rust实现跨平台开发(iOS/Android/Web)经验分享》核心精华
* **工具链标准化**：跨端开发最耗时的不是写代码，而是环境配置与多架构目标维护。必须在早期统一 `cargo-ndk` 及各端 Target，避免个人本地环境碎片化。
* **规避手动 C-FFI**：传统手写 `extern "C"` 容易出现内存泄漏、悬垂指针（Dangling Pointer）与段错误（SIGSEGV），且无法表达 Rust 富类型（如 `Result<T, E>`、`Option<T>`、复杂枚举）。推荐使用声明式绑定生成器（如 Mozilla UniFFI）。
* **异步与并发解耦**：Rust 的 `tokio` 或标准线程池适合处理后台长耗时计算，但**绝不要让 Rust 线程直接阻塞移动端主线程**；推荐通过回调（Callback）或通道（Channel）将结果异步投递回 UI 线程。

### 2. 《Rust 与 Android 桥接：高性能应用开发》核心精华
* **粗粒度接口（Coarse-Grained API）**：
  > [!WARNING]
  > **高频跨语言边界调用是性能杀手！**  
  > 绝对不要在 Kotlin 的遍历循环（`for-each`）中单次调用 Rust。跨越 JNI 边界有固定的上下文切换开销。必须将整个数据集打包传进 Rust，在 Rust 的高性能连续内存空间内算完后一次性返回。
* **零拷贝（Zero-Copy）内存共享**：
  * 对于图片处理、音视频帧、大型二进制流，严禁在 Java/Kotlin 堆与 Rust 堆之间反复拷贝字节数组。
  * 推荐在 Android 侧分配 `ByteBuffer.allocateDirect()`，直接将底层内存指针对齐给 Rust 处理，实现 0 开销共享。
* **生命周期与堆内存回收**：
  * 若 Rust 侧创建了长期存活的结构体对象并通过裸指针（`Box::into_raw`）传给 Android，Android 必须持有一个包装句柄，且在组件生命周期结束（如 `onDestroy()`）时**显式调用析构函数通知 Rust 释放**，否则会导致堆内存静默泄漏。

---

## 三、Android 端实战方案：UniFFI 现代化桥接

[Mozilla UniFFI](https://github.com/mozilla/uniffi-rs) 是目前经过 Firefox 亿级日活检验的跨平台多语言绑定标准。

### 1. 工作原理
在 Rust 中编写逻辑，通过过程宏声明对外暴露的接口，构建时 UniFFI 会自动生成符合 Kotlin 惯用法的客户端代码，自动处理序列化、错误转换与内存安全释放。

### 2. 代码范例

**Rust 核心层 (`shared_core/src/lib.rs`)**：
```rust
uniffi::setup_scaffolding!();

#[derive(uniffi::Record)]
pub struct UserProfile {
    pub id: u64,
    pub username: String,
    pub is_vip: bool,
}

#[derive(uniffi::Error, thiserror::Error, Debug)]
pub enum CoreError {
    #[error("Network error: {message}")]
    NetworkError { message: String },
    #[error("Invalid credentials")]
    AuthFailed,
}

#[uniffi::export]
pub fn fetch_user_data(token: String) -> Result<UserProfile, CoreError> {
    if token.is_empty() {
        return Err(CoreError::AuthFailed);
    }
    // 业务逻辑...
    Ok(UserProfile {
        id: 10001,
        username: "AntigravityUser".to_string(),
        is_vip: true,
    })
}
```

**Kotlin / Android UI 层直接调用**：
```kotlin
// UniFFI 自动生成的包装类，原生异常捕获机制：
try {
    val user: UserProfile = fetchUserData("user_token_abc")
    println("User: ${user.username}, VIP: ${user.isVip}")
} catch (e: CoreError.AuthFailed) {
    // 原生 Kotlin 异常处理
    showToast("认证失败，请重新登录")
}
```

---

## 四、HarmonyOS NEXT（纯血鸿蒙）实战方案：ohos-rs (Node-API)

纯血鸿蒙系统不再依赖 JVM，其应用层采用 ArkTS，底层 JavaScript 引擎为方舟运行时（ArkRuntime）。

### 1. 架构方案：基于 Node-API 的 `ohos-rs`
ArkRuntime 原生支持 Node-API 规范。社区由 OpenHarmony SIG 推出的 **[`ohos-rs`](https://github.com/ohos-rs/ohos-rs)**（原 `napi-rs` 的鸿蒙分支与适配套件），可以通过与 `napi-rs` 几乎一致的体验，直接把 Rust 代码导出为 ArkTS 模块。

### 2. 代码范例

**Rust 侧 (`shared_ohos/src/lib.rs`)**：
```rust
use napi_derive_ohos::napi;

#[napi(object)]
pub struct ArkUserProfile {
    pub id: i64,
    pub username: String,
    pub is_vip: bool,
}

#[napi]
pub fn calculate_hash(data: String) -> String {
    // 纯 Rust 高性能哈希计算...
    format!("hash_{}", data.len())
}

#[napi]
pub fn get_user_profile() -> ArkUserProfile {
    ArkUserProfile {
        id: 10001,
        username: "HarmonyNEXTUser".to_string(),
        is_vip: true,
    }
}
```

**编译与导出**：
1. 编译目标：`cargo build --target aarch64-unknown-linux-ohos --release`
2. 输出：`libshared_ohos.so`
3. 复制到 DevEco Studio 工程中的 `entry/libs/arm64-v8a/` 目录下。
4. 生成对应的 TypeScript 类型定义文件 `index.d.ts`。

**鸿蒙 ArkTS 界面调用**：
```typescript
import core from 'libshared_ohos.so';

@Entry
@Component
struct Index {
  @State username: string = '未获取'

  build() {
    Column() {
      Button('从 Rust 获取用户信息')
        .onClick(() => {
          const profile = core.getUserProfile();
          this.username = profile.username; // 直接访问结构体属性
        })
      Text(`当前用户: ${this.username}`)
    }
  }
}
```

---

## 五、全网生产级“踩坑”与避坑指南（关键干货）

综合全网开发者（包括腾讯、字节、开源社区）在移动端落地 Rust 遇到的血泪教训：

### 踩坑 1：避免 C/C++ 传递依赖（极度推荐 `rustls`）
* **大坑**：引入需要底层 C 库编译的 crate（如默认使用 OpenSSL 的 `reqwest`、`ring`、`zlib-sys` 等）。在交叉编译 Android 和鸿蒙 NEXT 时，NDK 头文件路径经常对不上，报错 `missing openssl/ssl.h` 或 `cc not found`。
* **解法**：**尽可能使用 100% 纯 Rust 实现的库**！
  * 网络请求：`reqwest` 关闭默认特性，启用 `rustls-tls`：
    ```toml
    reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
    ```
  * 压缩算法：使用纯 Rust 的 `flate2 = { version = "1.0", features = ["rust_backend"] }`。
  * 数据库：使用带有纯 Rust 绑定的 `rusqlite = { version = "0.31", features = ["bundled"] }`。

### 踩坑 2：统一跨平台日志（Logcat + HiLog + 控制台）
* **大坑**：在 Rust 核心层里写 `println!` 或 `eprintln!`，在移动端真机上**完全看不到任何输出**。
* **解法**：使用 Rust 统一的 `log` 或 `tracing` 门面（Facade），按平台分发：
  * Android 平台：挂载 `android_logger` crate，自动将 `log::info!` 转发至系统 `Logcat`。
  * 鸿蒙 NEXT 平台：挂载社区 `hilog-sys`，将日志直接输出至系统 `HiLog`（通过 `hdc hilog` 查看）。
  * 桌面预览：直接输出到终端 `env_logger`。

### 踩坑 3：编译产物体积暴增（从 20MB 缩减至 1.5MB）
默认的 Rust `release` 编译配置为了兼顾栈回溯和调试，保留了大量无用符号和元数据。必须在项目顶层 `Cargo.toml` 中配置：

```toml
[profile.release]
opt-level = "z"       # 体积优先优化
lto = true            # 开启跨 Crate 链接时优化 (消除无用死代码)
codegen-units = 1     # 提升编译器优化彻底程度
strip = true          # 彻底剔除符号表和调试数据
panic = "abort"       # 移动端不需要沉重的堆栈展开，崩溃直接 abort，显著减小体积
```

### 踩坑 4：异步 Runtime 的轻量化
* **大坑**：直接在移动端起一个 `tokio` 多线程完整运行时，容易吃满移动端核心并引发休眠竞争。
* **解法**：移动端后台任务通常并发量不高，如果必须使用异步：
  * 推荐配置 `tokio` 为单线程当前驱动：`tokio = { version = "1", features = ["rt", "macros"] }`（使用 `current_thread` 调度模式）。
  * 或者直接使用轻量级异步运行时（如 `smol` 或 `async-executor`）。

---

## 六、推荐项目结构与工程骨架

建议在 `E:\Rust_app` 下规划如下工程布局：

```text
E:\Rust_app\
├── activate.bat                   # 命令行快速激活环境
├── activate.ps1
├── Cargo.toml                     # Workspace 顶层配置
├── env\                           # [自包含环境] SDK、编译器、缓存
│   ├── rust\
│   ├── jdk-17\
│   ├── android\
│   └── harmony\
├── crates\
│   ├── app_core\                  # 【纯业务核心】与平台无关 (数据、业务算法、网络)
│   ├── bridge_android\            # 【Android 桥接层】使用 UniFFI 导出
│   └── bridge_harmony\            # 【鸿蒙 NEXT 桥接层】使用 napi-ohos 导出
├── apps\
│   ├── android_app\               # Android Studio 工程 (Kotlin / Jetpack Compose)
│   └── harmony_app\               # DevEco Studio 工程 (ArkTS / ArkUI)
└── tests\
    └── cli_preview\               # 【桌面快速预览】纯 Rust 命令行/轻量 UI 测试入口
```

---

### 总结要诀

1. **逻辑全下沉**：UI 只要动脑想怎么排版，复杂状态和数据全交由 Rust `app_core`。
2. **桥接全自动**：Android 用 **UniFFI**，鸿蒙用 **`ohos-rs`**，坚决不手写胶水代码。
3. **依赖全选纯 Rust**：优先挑选纯 Rust 库（如 `rustls`），避开 C/C++ 工具链兼容噩梦。
4. **编译先看 PC**：先在 PC 本地把单元测试跑绿，再去手机真机上验证，调试效率提升 10 倍。
