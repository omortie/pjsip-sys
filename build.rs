extern crate bindgen;

use os_info;

use std::collections::HashSet;
use std::fs::File;
use std::process::Command;
use std::{env, fs};
use std::path::{Path, PathBuf};
use zip::ZipArchive;

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

    //2. Create config_site.h
    create_config();

    //3. Get the pre-compiled PJPROJECT binaries from Github Releases
    let base = "https://github.com/omortie/pjsip-sys/releases/download/pre-compiled";

    //4. Determine OS and Link Libraries Accordingly
    let info = os_info::get();
    if info.os_type() == os_info::Type::Windows {
        let url = format!("{}/libpjproject-x86_64-x64-vc14-Release.zip", base);
        download_and_extract(&url);
        link_libs_windows();
    } else if (info.os_type() == os_info::Type::Linux) || (info.os_type() == os_info::Type::Ubuntu)
    {
        let url = format!("{}/pjproject-x86_64-pc-linux-gnu.zip", base);
        download_and_extract(&url);
        link_libs_linux();
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

    //5. Produce bindings.rs file
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

fn download_and_extract(url: &str) {
    // download
    let out_dir = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("pjlibs");
    fs::create_dir_all(&out_dir).unwrap();
    let archive_path = out_dir.join("pre-compiled.zip");

    if !archive_path.exists() {    
        let bytes = reqwest::blocking::get(url).unwrap().bytes().unwrap();
        fs::write(&archive_path, &bytes).unwrap();
    }

    // extract ZIP file
    let file = std::fs::File::open(&archive_path).unwrap();
    let mut archive = ZipArchive::new(file).unwrap();
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = match file.enclosed_name() {
            Some(path) => out_dir.join(path),
            None => continue,
        };

        if (*file.name()).ends_with('/') {
            // Directory
            fs::create_dir_all(&outpath).unwrap();
        } else {
            // File
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).unwrap();
                }
            }
            let mut outfile = fs::File::create(&outpath).unwrap();
            std::io::copy(&mut file, &mut outfile).unwrap();
        }
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

// WINDOWS
fn link_libs_windows() {
    let project_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    // The compiled libraries have been copied out of PJPROJECT to pjproject-sys/pjlibs/
    println!("cargo:rustc-link-search={}/pjlibs", project_dir);
    println!("cargo:rustc-link-lib=static=libpjproject-x86_64-x64-vc14-Release");
}

// LINUX
fn link_libs_linux() {
    let project_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let s = link_triple();
    let t = "static=";

    //Libraries inside PJPROJECT
    println!("cargo:rustc-link-search={}/pjlibs", project_dir);
    println!("cargo:rustc-link-lib={}pjsua{}", t, s);
    println!("cargo:rustc-link-lib={}pjsip{}", t, s);
    println!("cargo:rustc-link-lib={}pjsip-simple{}", t, s);
    println!("cargo:rustc-link-lib={}pjsua2{}", t, s);
    println!("cargo:rustc-link-lib={}pjsip-ua{}", t, s);

    println!("cargo:rustc-link-lib={}pj{}", t, s);

    println!("cargo:rustc-link-lib={}pjmedia{}", t, s);
    println!("cargo:rustc-link-lib={}pjmedia-codec{}", t, s);
    println!("cargo:rustc-link-lib={}pjmedia-videodev{}", t, s);
    println!("cargo:rustc-link-lib={}pjmedia-audiodev{}", t, s);

    println!("cargo:rustc-link-lib={}pjnath{}", t, s);

    println!("cargo:rustc-link-lib={}pjlib-util{}", t, s);

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
