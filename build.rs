use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let profile = env::var("PROFILE").unwrap(); // "debug" or "release"
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let (subdir, filename): (&str, &str) = match target_os.as_str() {
        "windows" => ("windows", "steam_api64.dll"),
        "macos" => ("macos", "libsteam_api.dylib"),
        "linux" => ("linux", "libsteam_api.so"),
        other => panic!("Unsupported target OS for Steamworks redistributable: {other}"),
    };

    let redist_dir = PathBuf::from(&manifest_dir).join("steam-redist");
    let dest_dir = PathBuf::from(&manifest_dir).join("target").join(&profile);
    fs::create_dir_all(&dest_dir).expect("Failed to create target dir");

    // Platform-specific Steamworks shared library
    let lib_src = redist_dir.join(subdir).join(filename);
    if lib_src.exists() {
        fs::copy(&lib_src, dest_dir.join(filename)).expect("Failed to copy Steamworks redistributable");
        println!("cargo:rerun-if-changed={}", lib_src.display());
    } else {
        println!(
            "cargo:warning=Steam redistributable not found at {} — Steam features will fail to init.",
            lib_src.display()
        );
    }

    // steam_appid.txt
    let appid_src = redist_dir.join("steam_appid.txt");
    if appid_src.exists() {
        fs::copy(&appid_src, dest_dir.join("steam_appid.txt")).expect("Failed to copy steam_appid.txt");
        println!("cargo:rerun-if-changed={}", appid_src.display());
    } else {
        println!(
            "cargo:warning=steam_appid.txt not found at {} — Client::init_app may fail without it.",
            appid_src.display()
        );
    }
}