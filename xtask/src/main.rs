use std::path::PathBuf;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let aura_verb_dir = manifest_dir.parent().unwrap();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let is_release = args.iter().any(|a| a == "--release");

    let mut cmd = Command::new("cargo");
    cmd.current_dir(aura_verb_dir).arg("build");
    if is_release {
        cmd.arg("--release");
    }

    let status = cmd.status()?;
    if !status.success() {
        return Err("cargo build failed".into());
    }

    let profile = if is_release { "release" } else { "debug" };
    let so_path = aura_verb_dir.join("target").join(profile).join("libaura_verb.so");
    let bundled_dir = aura_verb_dir.join("target").join("bundled");
    std::fs::create_dir_all(&bundled_dir)?;
    let clap_path = bundled_dir.join("aura-verb.clap");

    if so_path.exists() {
        std::fs::copy(&so_path, &clap_path)?;
        println!("Successfully created bundle at '{}'", clap_path.display());
    }

    Ok(())
}
