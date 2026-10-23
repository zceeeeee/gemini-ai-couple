@echo off
set CARGO_HOME=E:\Rust_app\env\rust\.cargo
set RUSTUP_HOME=E:\Rust_app\env\rust\.rustup
set GRADLE_USER_HOME=E:\Rust_app\env\android\.gradle
set JAVA_HOME=E:\Rust_app\env\jdk-17
set ANDROID_HOME=E:\Rust_app\env\android\sdk
set ANDROID_SDK_ROOT=E:\Rust_app\env\android\sdk
set ANDROID_NDK_HOME=E:\Rust_app\env\android\sdk\ndk\26.3.11579264
set NDK_HOME=E:\Rust_app\env\android\sdk\ndk\26.3.11579264
set OHOS_BASE_SDK_HOME=E:\Rust_app\env\harmony\sdk
set DEVECO_SDK_HOME=E:\Rust_app\env\harmony\sdk

set PATH=E:\Rust_app\env\rust\.cargo\bin;E:\Rust_app\env\jdk-17\bin;E:\Rust_app\env\android\sdk\platform-tools;E:\Rust_app\env\android\sdk\cmdline-tools\latest\bin;E:\Rust_app\env\harmony\sdk\toolchains;E:\Rust_app\env\harmony\sdk\native\llvm\bin;%PATH%

echo ==========================================================
echo   [Rust_app] Dual-Platform Environment Activated!
echo ==========================================================
echo   CARGO_HOME         : %CARGO_HOME%
echo   RUSTUP_HOME        : %RUSTUP_HOME%
echo   JAVA_HOME          : %JAVA_HOME%
echo   ANDROID_HOME       : %ANDROID_HOME%
echo   ANDROID_NDK_HOME   : %ANDROID_NDK_HOME%
echo   OHOS_BASE_SDK_HOME : %OHOS_BASE_SDK_HOME%
echo ==========================================================
