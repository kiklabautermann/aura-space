use std::path::PathBuf;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let aura_space_dir = manifest_dir.parent().unwrap();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let is_release = args.iter().any(|a| a == "--release");

    let is_bundle_only = args.iter().any(|a| a == "--no-build");

    if !is_bundle_only {
        let mut cmd = Command::new("cargo");
        cmd.current_dir(aura_space_dir).arg("build");
        if is_release {
            cmd.arg("--release");
        }

        let status = cmd.status()?;
        if !status.success() {
            return Err("cargo build failed".into());
        }
    }

    let profile = if is_release { "release" } else { "debug" };
    let bundled_dir = aura_space_dir.join("target").join("bundled");
    std::fs::create_dir_all(&bundled_dir)?;

    if cfg!(target_os = "macos") {
        // macOS bundle layout: aura-space.clap/Contents/MacOS/aura-space
        let bundle_root = bundled_dir.join("aura-space.clap");
        let contents_dir = bundle_root.join("Contents");
        let macos_dir = contents_dir.join("MacOS");
        std::fs::create_dir_all(&macos_dir)?;

        let dylib_path = aura_space_dir.join("target").join(profile).join("libaura_space.dylib");
        if dylib_path.exists() {
            std::fs::copy(&dylib_path, macos_dir.join("aura-space"))?;
        }

        std::fs::write(contents_dir.join("PkgInfo"), "BNDL????")?;

        let plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>aura-space</string>
    <key>CFBundleIdentifier</key>
    <string>com.aura.auraspace</string>
    <key>CFBundleName</key>
    <string>AuraSpace</string>
    <key>CFBundlePackageType</key>
    <string>BNDL</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CSResourcesFileMapped</key>
    <true/>
</dict>
</plist>"#;
        std::fs::write(contents_dir.join("Info.plist"), plist)?;
        println!("Successfully created macOS bundle at '{}'", bundle_root.display());
    } else {
        // Linux: direct .clap shared library
        let so_path = aura_space_dir.join("target").join(profile).join("libaura_space.so");
        let clap_path = bundled_dir.join("aura-space.clap");

        if so_path.exists() {
            std::fs::copy(&so_path, &clap_path)?;
            println!("Successfully created Linux bundle at '{}'", clap_path.display());
        }
    }

    Ok(())
}
