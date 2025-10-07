extern crate bindgen;

use os_info;

use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug)]
struct IgnoreMacros(HashSet<String>);

impl bindgen::callbacks::ParseCallbacks for IgnoreMacros {
    fn will_parse_macro(&self, name: &str) -> bindgen::callbacks::MacroParsingBehavior {
        if self.0.contains(name) {
            bindgen::callbacks::MacroParsingBehavior::Ignore
        } else {
            bindgen::callbacks::MacroParsingBehavior::Default
        }
    }
}

// WINDOWS
fn link_libs_windows() {
    let project_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    // The compiled libraries have been copied out of PJPROJECT to pjproject-sys/pjlibs/
    println!("cargo:rustc-link-search={}/pjlibs/windows", project_dir);
    println!("cargo:rustc-link-lib=static=libpjproject-x86_64-x64-vc14-Release");
}

fn main() {
    pkg_config::Config::new().probe("libpjproject").unwrap();
    println!("cargo:rerun-if-changed=wrapper.h");

    let ignored_macros = IgnoreMacros(
        vec![
            "FP_NORMAL".into(),
            "FP_SUBNORMAL".into(),
            "FP_ZERO".into(),
            "FP_INFINITE".into(),
            "FP_NAN".into(),
            "IPPORT_RESERVED".into(),
        ]
        .into_iter()
        .collect(),
    );

    let bindings = bindgen::Builder::default()
        .clang_arg("-I./pjproject/pjlib/include")
        .clang_arg("-I./pjproject/pjsip/include")
        .clang_arg("-I./pjproject/pjlib-util/include")
        .clang_arg("-I./pjproject/pjmedia/include")
        .clang_arg("-I./pjproject/pjnath/include")
        .header("wrapper.h")
        .parse_callbacks(Box::new(ignored_macros))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    //1. Fetch pjproject
    let curr_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    Command::new("git")
        .arg("submodule")
        .arg("update")
        .arg("--init")
        .current_dir(&curr_dir)
        .status()
        .unwrap();

    //2. Create config_site.h (so bindgen doesnt complain about missing header files)
    let file = File::create("pjproject/pjlib/include/pj/config_site.h");
    match file {
        Ok(_x) => println!("config_site.h created"),
        Err(_x) => {
            println!("config_site.h not created, Error!");
            panic!("config_site.h not created, Error!");
        }
    };

    //3. Determine OS and Link Libraries Accordingly
    let info = os_info::get();
    if info.os_type() == os_info::Type::Windows {
        link_libs_windows();
    } else if (info.os_type() == os_info::Type::Linux) || (info.os_type() == os_info::Type::Ubuntu)
    {
        link_libs_unix();
    }
}

fn link_libs_unix() {
    let project_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    println!("cargo:rustc-link-search={}/pjlibs/linux", project_dir);

    //Libraries inside PJPROJECT
    println!("cargo:rustc-link-lib=pjsua-x86_64-pc-linux-gnu");
    println!("cargo:rustc-link-lib=pjsip-x86_64-pc-linux-gnu");
    println!("cargo:rustc-link-lib=pj-x86_64-pc-linux-gnu");
    println!("cargo:rustc-link-lib=pjsip-simple-x86_64-pc-linux-gnu");
    println!("cargo:rustc-link-lib=pjsua2-x86_64-pc-linux-gnu");
    println!("cargo:rustc-link-lib=pjsip-ua-x86_64-pc-linux-gnu");
    println!("cargo:rustc-link-lib=pjmedia-codec-x86_64-pc-linux-gnu");
    println!("cargo:rustc-link-lib=pjmedia-videodev-x86_64-pc-linux-gnu");
    println!("cargo:rustc-link-lib=pjmedia-audiodev-x86_64-pc-linux-gnu");

    println!("cargo:rustc-link-lib=pjmedia-x86_64-pc-linux-gnu");
    println!("cargo:rustc-link-lib=pjnath-x86_64-pc-linux-gnu");
    println!("cargo:rustc-link-lib=pjlib-util-x86_64-pc-linux-gnu");
    println!("cargo:rustc-link-lib=yuv-x86_64-pc-linux-gnu");
    println!("cargo:rustc-link-lib=ilbccodec-x86_64-pc-linux-gnu");
    println!("cargo:rustc-link-lib=g7221codec-x86_64-pc-linux-gnu");
    println!("cargo:rustc-link-lib=gsmcodec-x86_64-pc-linux-gnu");
    println!("cargo:rustc-link-lib=resample-x86_64-pc-linux-gnu");
    println!("cargo:rustc-link-lib=srtp-x86_64-pc-linux-gnu");
    println!("cargo:rustc-link-lib=webrtc-x86_64-pc-linux-gnu");
    println!("cargo:rustc-link-lib=speex-x86_64-pc-linux-gnu");

    // Dependencies
    println!("cargo:rustc-link-lib=ssl");
    println!("cargo:rustc-link-lib=crypto");
    println!("cargo:rustc-link-lib=z");
    println!("cargo:rustc-link-lib=asound");
    println!("cargo:rustc-link-lib=uuid");
}
