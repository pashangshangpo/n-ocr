fn main() {
    #[cfg(target_os = "macos")]
    {
        let mut build = cc::Build::new();
        build.file("src/ocr_bridge.swift");
        build.flag("-import-objc-header");
        build.flag("src/ocr_bridge.h");

        let target = std::env::var("TARGET").unwrap();
        let sdk = if target.contains("ios") {
            "iphoneos"
        } else {
            "macosx"
        };

        let sdk_path = std::process::Command::new("xcrun")
            .args(["--sdk", sdk, "--show-sdk-path"])
            .output()
            .expect("failed to get SDK path")
            .stdout;
        let sdk_path = String::from_utf8(sdk_path).unwrap();
        let sdk_path = sdk_path.trim();

        build.flag("-sdk");
        build.flag(sdk_path);
        build.flag("-target");

        let swift_target = if target.contains("aarch64") {
            "arm64-apple-macosx10.15"
        } else {
            "x86_64-apple-macosx10.15"
        };
        build.flag(swift_target);

        let swift_out = std::process::Command::new("swiftc")
            .args([
                "-emit-library",
                "-emit-module",
                "-module-name", "ocr_bridge",
                "-target", swift_target,
                "-sdk", sdk_path,
                "-import-objc-header", "src/ocr_bridge.h",
                "src/ocr_bridge.swift",
                "-o",
            ])
            .arg(format!("{}/libocr_bridge.a", std::env::var("OUT_DIR").unwrap()))
            .arg("-static")
            .output()
            .expect("failed to compile Swift");

        if !swift_out.status.success() {
            panic!(
                "Swift compilation failed:\n{}",
                String::from_utf8_lossy(&swift_out.stderr)
            );
        }

        let out_dir = std::env::var("OUT_DIR").unwrap();
        println!("cargo:rustc-link-search=native={}", out_dir);
        println!("cargo:rustc-link-lib=static=ocr_bridge");
        println!("cargo:rustc-link-lib=framework=Vision");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
        println!("cargo:rustc-link-lib=framework=Foundation");

        let swift_lib_paths = std::process::Command::new("swiftc")
            .args(["-print-target-info", "-target", swift_target])
            .output()
            .expect("failed to get swift target info");
        let info: serde_json::Value =
            serde_json::from_slice(&swift_lib_paths.stdout).expect("failed to parse swift info");
        if let Some(paths) = info["paths"]["runtimeLibraryPaths"].as_array() {
            for path in paths {
                if let Some(p) = path.as_str() {
                    println!("cargo:rustc-link-search=native={}", p);
                }
            }
        }
    }
}