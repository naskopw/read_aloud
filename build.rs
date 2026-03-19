extern crate cbindgen;

use current_platform::CURRENT_PLATFORM;
use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let config_path = PathBuf::from(&crate_dir).join("cbindgen.toml");

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(cbindgen::Config::from_file(config_path).expect("Failed to read cbindgen.toml"))
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(PathBuf::from(target_dir()).join("read_aloud.h"));
}

fn target_dir() -> PathBuf {
    let default_target = CURRENT_PLATFORM;
    let target_triple = env::var("TARGET").expect("TARGET environment variable not set");
    let is_default_target = target_triple == default_target;
    let mut path = env::var_os("CARGO_TARGET_DIR").map(PathBuf::from).unwrap_or_else(|| {
        let mut path = PathBuf::from(
            env::var("CARGO_MANIFEST_DIR")
                .expect("CARGO_MANIFEST_DIR environment variable not set"),
        );
        path.push("target");
        path
    });
    let profile = env::var("PROFILE").expect("PROFILE environment variable not set");
    if !is_default_target {
        path.push(target_triple);
    }
    path.push(profile);
    path
}
