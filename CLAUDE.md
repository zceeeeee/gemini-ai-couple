# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This repository is a self-contained, isolated development workspace for Rust cross-platform mobile applications targeting both **Android** and **HarmonyOS NEXT (纯血鸿蒙)**. It follows the **Shared Core** architectural pattern: core business logic resides in pure Rust, while UI layers are implemented using platform-native declarative frameworks (Jetpack Compose / ArkUI).

## Environment & Toolchains

All toolchains and SDKs are hermetically isolated within the `env/` directory to avoid system-level pollution:
- **Rust**: `env/rust/.cargo` (`CARGO_HOME`) and `env/rust/.rustup` (`RUSTUP_HOME`). Target linkers for OpenHarmony are preconfigured in `env/rust/.cargo/config.toml`.
- **Java**: `env/jdk-17` (`JAVA_HOME`, OpenJDK 17).
- **Android SDK & NDK**: `env/android/sdk` (`ANDROID_HOME`, `ANDROID_NDK_HOME`) with NDK `26.3.11579264`.
- **HarmonyOS SDK**: `env/harmony/sdk` (`OHOS_BASE_SDK_HOME`, `DEVECO_SDK_HOME`) including LLVM toolchains and `hdc`.
- **Preinstalled Rust Targets**:
  - Host: `x86_64-pc-windows-msvc`
  - Android: `aarch64-linux-android`, `armv7-linux-androideabi`, `x86_64-linux-android`
  - HarmonyOS NEXT: `aarch64-unknown-linux-ohos`, `x86_64-unknown-linux-ohos`

### Activation
Before running any Cargo, Rust, Android, or HarmonyOS commands, activate the isolated environment in the current shell session:
- **PowerShell**: `.\activate.ps1`
- **CMD**: `activate.bat`

## Common Commands

### Host Development & Verification
- Check code: `cargo check`
- Build workspace: `cargo build`
- Run all tests: `cargo test`
- Run a single test: `cargo test <test_name> -- --exact`
- Format check: `cargo fmt --check`
- Linter: `cargo clippy --all-targets`

### Android Cross-Compilation
- Build with `cargo-ndk` (outputs `.so` libraries for multiple ABIs):
  `cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -o <output_dir> build --release`
- Target-specific build:
  `cargo build --target aarch64-linux-android --release`

### HarmonyOS NEXT Cross-Compilation
- Build device target (ARM64):
  `cargo build --target aarch64-unknown-linux-ohos --release`
- Build emulator target (x86_64):
  `cargo build --target x86_64-unknown-linux-ohos --release`
- View device logs via HDC:
  `hdc hilog`

## High-Level Architecture & Project Blueprint

The repository structure follows the Shared Core layout documented in `RUST_MOBILE_CROSS_PLATFORM_GUIDE.md`:

```
E:\Rust_app\
├── activate.bat / activate.ps1     # Environment activation scripts
├── env\                           # Isolated SDKs & toolchains (git-ignored)
├── crates\
│   ├── app_core\                  # Pure business logic, data models, offline DB, network
│   ├── bridge_android\            # Android FFI bindings via Mozilla UniFFI (Kotlin wrapper)
│   └── bridge_harmony\            # HarmonyOS NEXT bindings via ohos-rs / Node-API (ArkTS)
├── apps\
│   ├── android_app\               # Android Studio project (Kotlin + Jetpack Compose)
│   └── harmony_app\               # DevEco Studio project (ArkTS + ArkUI)
└── tests\
    └── cli_preview\               # Fast local desktop CLI harness for logic validation
```

### Bridge Strategies
1. **Android (UniFFI)**: Uses Mozilla UniFFI (`uniffi::setup_scaffolding!`, `#[derive(uniffi::Record)]`, `#[uniffi::export]`) to generate type-safe Kotlin bindings with idiomatic exceptions and automatic memory cleanup.
2. **HarmonyOS NEXT (`ohos-rs`)**: Uses OpenHarmony Node-API bindings (`napi-derive-ohos`) to compile `.so` libraries and generate `index.d.ts` types consumable directly by ArkTS.
3. **Harmony Linkers**: Cargo uses `env/harmony/aarch64-clang.bat` and `x86_64-clang.bat` (configured in `env/rust/.cargo/config.toml`) to invoke Harmony's LLVM clang with appropriate `--sysroot` and `--target`.

## Architectural Rules & Key Constraints

- **Pure Rust Dependencies**: Avoid crates linking to system C/C++ libraries (e.g. OpenSSL, zlib-sys). Always prefer 100% Rust implementations:
  - Use `reqwest` with `default-features = false, features = ["rustls-tls"]`.
  - Use `rusqlite` with `features = ["bundled"]`.
  - Use `flate2` with `features = ["rust_backend"]`.
- **Coarse-Grained Bridge APIs**: Do not cross language boundaries inside UI loops. Batch data operations and transfer bulk data or use zero-copy buffers (`ByteBuffer.allocateDirect()` on Android) where applicable.
- **Lightweight Async**: On mobile targets, avoid multi-threaded `tokio` runtimes. Use `tokio` with `current_thread` flavor or lightweight runtimes like `smol`.
- **Unified Logging**: Never use unmanaged `println!`. Route logs through `log` / `tracing` facade to `android_logger` on Android and `hilog-sys` on HarmonyOS NEXT.
- **Binary Size Profile**: Configure release builds in workspace root `Cargo.toml`:
  ```toml
  [profile.release]
  opt-level = "z"
  lto = true
  codegen-units = 1
  strip = true
  panic = "abort"
  ```
