fn main() {
    // ScreenCaptureKit's Swift bridge needs the Swift runtime on the rpath.
    // The crate's own build.rs sets these, but cargo:rustc-link-arg from a
    // library dependency doesn't propagate to the final binary link step.
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
        if let Ok(xcode_path) = std::process::Command::new("xcode-select")
            .arg("-p")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        {
            println!(
                "cargo:rustc-link-arg=-Wl,-rpath,{}/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift/macosx",
                xcode_path
            );
            println!(
                "cargo:rustc-link-arg=-Wl,-rpath,{}/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift-5.5/macosx",
                xcode_path
            );
        }
    }

    tauri_build::build()
}
