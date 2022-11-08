use std::process::Command;
use std::env;
use std::path::{Path};

fn main() {

    let out_dir = env::var("OUT_DIR").unwrap();

    let out_dir_path = Path::new(&out_dir);
    let path = out_dir_path.join("libjit");

    // TODO: Windows support?

    Command::new("cp").args(&["-R", "./libjit", &out_dir]).output().expect("unable to copy libjit to outdir");

    env::set_current_dir(path.to_str().expect("expect libjit to exist in cargo outdir")).expect("expected libjit to exist in out dir");

    let bootstrap_path = path.join("bootstrap");
    let configure_path= path.join("configure");
    Command::new("chmod").args(&["+x", bootstrap_path.to_str().unwrap()]).output().expect("to make bootstrap executable");
    Command::new("chmod").args(&["+x", configure_path.to_str().unwrap()]).output().expect("to make bootstrap executable");
    Command::new("./bootstrap")
        .output().expect("Failed to run `./bootstrap` installing libjit");
    Command::new("./configure")
        .args(["--disable-dependency-tracking"])
        .output().expect("Failed to run `./configure --disable-dependency-tracking` while installing libjit");
    Command::new("make")
        .output().expect("Failed to run `make` while installing libjit");
    Command::new("make").args(&["install"])
        .output().expect("Failed to run `make install` while installing libjit");

    let library_path = path.join("jit").join(".libs");
    let jit_path = path.join("jit");

    env::set_current_dir(jit_path.to_str().expect("expect libs dir to exist in cargo outdir")).expect("expected libs dir to exist in out dir");
    Command::new("strip").args(&["-x", "*"])
        .output().expect("Failed to strip libjit linkable libraries");

    env::set_current_dir(library_path.to_str().expect("expect libs dir to exist in cargo outdir")).expect("expected libs dir to exist in out dir");

    // Shared library is too big so strip it
    Command::new("strip").args(&["-x", "*"])
        .output().expect("Failed to strip libjit linkable libraries");

    println!("cargo:rustc-link-search={}", library_path.to_str().expect("libjit './jit/.libs' directory was not created during installation for some reason"));
    println!("cargo:rustc-link-lib=static=jit");
    println!("cargo:rerun-if-changed=build.rs");
}
