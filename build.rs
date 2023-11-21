extern crate cbindgen;

use current_platform::CURRENT_PLATFORM;
use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut config: cbindgen::Config = Default::default();
    config.language = cbindgen::Language::C;

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(PathBuf::from(target_dir()).join("read_aloud.h"));
}

fn target_dir() -> PathBuf {
    // Determine the default target for the current platform
    let default_target = CURRENT_PLATFORM;
    let target_triple = env::var("TARGET").expect("TARGET environment variable not set");
    let is_default_target = target_triple == default_target;
    let mut path = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR environment variable not set"),
    );
    let profile = env::var("PROFILE").expect("PROFILE environment variable not set");
    path.push("target");
    if !is_default_target {
        path.push(target_triple);
    }
    path.push(profile);
    path
}
