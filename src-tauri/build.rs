fn main() {
    // Warn if no GPU acceleration feature is enabled
    #[cfg(not(any(
        feature = "enable-metal",
        feature = "enable-cuda",
        feature = "enable-vulkan"
    )))]
    {
        println!("cargo:warning=Building without GPU acceleration. Consider:");
        println!("cargo:warning=  macOS:   --features enable-metal");
        println!("cargo:warning=  Windows: --features enable-cuda or enable-vulkan");
        println!("cargo:warning=  Or use the platform-specific npm scripts:");
        println!("cargo:warning=    npm run tauri:build:macos");
        println!("cargo:warning=    npm run tauri:build:windows:cuda");
    }

    tauri_build::build()
}
