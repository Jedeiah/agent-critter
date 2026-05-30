fn main() {
    #[cfg(target_os = "windows")]
    {
        // wry needs these for WebView2 on Windows
        println!("cargo:rustc-link-lib=advapi32");
        println!("cargo:rustc-link-lib=ole32");
        println!("cargo:rustc-link-lib=shlwapi");
        println!("cargo:rustc-link-lib=shell32");
    }
}
