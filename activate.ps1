$env:CARGO_HOME = 'E:\Rust_app\env\rust\.cargo'
$env:RUSTUP_HOME = 'E:\Rust_app\env\rust\.rustup'
$env:GRADLE_USER_HOME = 'E:\Rust_app\env\android\.gradle'
$env:JAVA_HOME = 'E:\Rust_app\env\jdk-17'
$env:ANDROID_HOME = 'E:\Rust_app\env\android\sdk'
$env:ANDROID_SDK_ROOT = 'E:\Rust_app\env\android\sdk'
$env:ANDROID_NDK_HOME = 'E:\Rust_app\env\android\sdk\ndk\26.3.11579264'
$env:NDK_HOME = 'E:\Rust_app\env\android\sdk\ndk\26.3.11579264'
$env:OHOS_BASE_SDK_HOME = 'E:\Rust_app\env\harmony\sdk'
$env:DEVECO_SDK_HOME = 'E:\Rust_app\env\harmony\sdk'

$extraPaths = @(
    'E:\Rust_app\env\rust\.cargo\bin',
    'E:\Rust_app\env\jdk-17\bin',
    'E:\Rust_app\env\android\sdk\platform-tools',
    'E:\Rust_app\env\android\sdk\cmdline-tools\latest\bin',
    'E:\Rust_app\env\harmony\sdk\toolchains',
    'E:\Rust_app\env\harmony\sdk\native\llvm\bin'
)

foreach ($p in $extraPaths) {
    if ($env:Path -notlike "*$p*") {
        $env:Path = "$p;$env:Path"
    }
}

Write-Host "==========================================================" -ForegroundColor Green
Write-Host "  [Rust_app] Dual-Platform Environment Activated!         " -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Green
Write-Host "CARGO_HOME         : $env:CARGO_HOME"
Write-Host "RUSTUP_HOME        : $env:RUSTUP_HOME"
Write-Host "JAVA_HOME          : $env:JAVA_HOME"
Write-Host "ANDROID_HOME       : $env:ANDROID_HOME"
Write-Host "ANDROID_NDK_HOME   : $env:ANDROID_NDK_HOME"
Write-Host "OHOS_BASE_SDK_HOME : $env:OHOS_BASE_SDK_HOME"
Write-Host "==========================================================" -ForegroundColor Green
