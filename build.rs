extern crate bindgen;
extern crate cc;
extern crate make_cmd;

use os_info;

use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::path::{Path, PathBuf};
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

fn main() {
    println!("cargo:rerun-if-changed=wrapper.h");

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
    create_config();

    //3. Determine OS and Link Libraries Accordingly
    let info = os_info::get();
    if info.os_type() == os_info::Type::Windows {
        link_libs_windows();
    } else if (info.os_type() == os_info::Type::Linux) || (info.os_type() == os_info::Type::Ubuntu)
    {
        if check_pj_built_status().is_none() {
            compile_pj();
        }
        link_libs_unix();
    }

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

    //4. Produce bindings.rs file
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
}

fn create_config() {
    let file = File::create("pjproject/pjlib/include/pj/config_site.h");
    match file {
        Ok(_x) => println!("config_site.h created"),
        Err(_x) => {
            println!("config_site.h not created, Error!");
            panic!("config_site.h not created, Error!");
        }
    };
}

fn check_path(p: String) -> Option<()> {
    if Path::new(&p).exists() {
        Some(())
    } else {
        None
    }
}

fn real_env() -> String {
    let target = env::var("TARGET").unwrap();
    let s: Vec<&str> = target.split_terminator("-").collect();
    s.get(s.len() - 1).unwrap().to_string()
}

fn link_triple() -> String {
    format!("-{}-{}-{}-{}",
            env::var("CARGO_CFG_TARGET_ARCH").unwrap(),
            "pc",
            env::var("CARGO_CFG_TARGET_OS").unwrap(),
            real_env()
    )
}

fn check_pj_built_status() -> Option<()> {
    let lt = format!("{}.a", link_triple());

    check_path(format!("pjproject/third_party/lib/libg7221codec{}", lt))?;
    check_path(format!("pjproject/third_party/lib/libgsmcodec{}", lt))?;
    check_path(format!("pjproject/third_party/lib/libilbccodec{}", lt))?;
    check_path(format!("pjproject/third_party/lib/libresample{}", lt))?;
    check_path(format!("pjproject/third_party/lib/libspeex{}", lt))?;
    check_path(format!("pjproject/third_party/lib/libsrtp{}", lt))?;

    check_path(format!("pjproject/pjlib/lib/libpj{}", lt))?;

    check_path(format!("pjproject/pjlib-util/lib/libpjlib-util{}", lt))?;

    check_path(format!("pjproject/pjnath/lib/libpjnath{}", lt))?;

    check_path(format!("pjproject/pjmedia/lib/libpjmedia{}", lt))?;
    check_path(format!("pjproject/pjmedia/lib/libpjmedia-codec{}", lt))?;

    check_path(format!("pjproject/pjmedia/lib/libpjmedia-audiodev{}", lt))?;
    check_path(format!("pjproject/pjmedia/lib/libpjsdp{}", lt))?;

    check_path(format!("pjproject/pjsip/lib/libpjsip{}", lt))?;
    check_path(format!("pjproject/pjsip/lib/libpjsip-simple{}", lt))?;
    check_path(format!("pjproject/pjsip/lib/libpjsip-ua{}", lt))?;
    check_path(format!("pjproject/pjsip/lib/libpjsua{}", lt))
}

fn compile_pj() {
    let mut c = Command::new("sh");

    c.current_dir("./pjproject/");
    c.spawn().unwrap().wait().unwrap();

    make_cmd::make()
        .arg("dep")
        .current_dir("./pjproject/")
        .spawn().unwrap().wait().unwrap();

    make_cmd::make()
        .current_dir("./pjproject/")
        .spawn().unwrap().wait().unwrap();
}


// WINDOWS
fn link_libs_windows() {
    let project_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    // The compiled libraries have been copied out of PJPROJECT to pjproject-sys/pjlibs/
    println!("cargo:rustc-link-search={}/pjlibs/windows", project_dir);
    println!("cargo:rustc-link-lib=static=libpjproject-x86_64-x64-vc14-Release");
}

fn link_libs_unix() {
    let s = link_triple();
    let t = "static=";

    //Libraries inside PJPROJECT
    println!("cargo:rustc-link-search=pjproject/pjsip/lib/");
    println!("cargo:rustc-link-lib={}pjsua{}", t, s);
    println!("cargo:rustc-link-lib={}pjsip{}", t, s);
    println!("cargo:rustc-link-lib={}pjsip-simple{}", t, s);
    println!("cargo:rustc-link-lib={}pjsua2{}", t, s);
    println!("cargo:rustc-link-lib={}pjsip-ua{}", t, s);

    println!("cargo:rustc-link-search=pjproject/pjlib/lib/");
    println!("cargo:rustc-link-lib={}pj{}", t, s);
    
    println!("cargo:rustc-link-search=pjproject/pjmedia/lib/");
    println!("cargo:rustc-link-lib={}pjmedia{}", t, s);
    println!("cargo:rustc-link-lib={}pjmedia-codec{}", t, s);
    println!("cargo:rustc-link-lib={}pjmedia-videodev{}", t, s);
    println!("cargo:rustc-link-lib={}pjmedia-audiodev{}", t, s);

    println!("cargo:rustc-link-search=pjproject/pjnath/lib/");
    println!("cargo:rustc-link-lib={}pjnath{}", t, s);

    println!("cargo:rustc-link-search=pjproject/pjlib-util/lib/");
    println!("cargo:rustc-link-lib={}pjlib-util{}", t, s);

    println!("cargo:rustc-link-search=pjproject/third_party/lib/");
    println!("cargo:rustc-link-lib={}gsmcodec{}", t, s);
    println!("cargo:rustc-link-lib={}resample{}", t, s);
    println!("cargo:rustc-link-lib={}srtp{}", t, s);
    println!("cargo:rustc-link-lib={}speex{}", t, s);
    println!("cargo:rustc-link-lib={}ilbccodec{}", t, s);
    println!("cargo:rustc-link-lib={}g7221codec{}", t, s);

    // Dependencies
    println!("cargo:rustc-link-lib=ssl");
    println!("cargo:rustc-link-lib=crypto");
    println!("cargo:rustc-link-lib=z");
    println!("cargo:rustc-link-lib=asound");
    println!("cargo:rustc-link-lib=uuid");
}
