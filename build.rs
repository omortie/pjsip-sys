extern crate bindgen;

use os_info;

use std::collections::HashSet;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};
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

fn get_target_info() -> (String, String, String) {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    // let target_vendor = env::var("CARGO_CFG_TARGET_VENDOR").unwrap_or("unknown".to_string());
    let vendor = if target_arch == "x86_64" {
        "pc".to_string()
    } else {
        "unknown".to_string()
    };
    (target_os, vendor, target_arch)
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

    //2. Create config_site.h necessary for pjproject linking
    create_config();

    //3. Get the pre-compiled PJPROJECT binaries from Github Releases
    let base = "https://github.com/omortie/pjsip-sys/releases/download/pre-compiled";

    //4. Determine OS and Link Libraries Accordingly
    let info = os_info::get();
    let (target_os, vendor, target_arch) = get_target_info();

    if info.os_type() == os_info::Type::Windows {
        let url = format!("{}/{}-x64-vc14-Release.zip", base, target_arch);
        download_and_extract(&url);
        configure_windows();
    } else if (info.os_type() == os_info::Type::Linux) || (info.os_type() == os_info::Type::Ubuntu)
    {
        if target_os == "android" {
            let url = format!(
                "{}/{}-{}-linux-{}.zip",
                base, target_arch, vendor, target_os
            );
            println!("Downloading from URL: {}", url);
            download_and_extract(&url);
            configure_android();
        } else {
            let url = format!("{}/{}-{}-linux-gnu.zip", base, target_arch, vendor);
            download_and_extract(&url);
            configure_linux();
        }
    }
}

fn configure_android() {
    link_libs_android();
    generate_bindings_android();
}

fn configure_linux() {
    link_libs_linux();
    generate_bindings_default();
}

fn configure_windows() {
    link_libs_windows();
    generate_bindings_default();
}

fn generate_bindings_android() {
    let (_, __, target_arch) = get_target_info();

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

    let mut builder = bindgen::Builder::default()
        .clang_arg("-I./pjproject/pjlib/include")
        .clang_arg("-I./pjproject/pjsip/include")
        .clang_arg("-I./pjproject/pjlib-util/include")
        .clang_arg("-I./pjproject/pjmedia/include")
        .clang_arg("-I./pjproject/pjnath/include")
        .header("wrapper.h")
        .parse_callbacks(Box::new(ignored_macros));

    // Define Android-specific macros
    builder = builder
        .clang_arg("-DANDROID=1")
        .clang_arg("-D_FORTIFY_SOURCE=2")
        .clang_arg("-D__ANDROID_UNAVAILABLE_SYMBOLS_ARE_WEAK__")
        .clang_arg("-DPJ_IS_LITTLE_ENDIAN=1")
        .clang_arg("-DPJ_IS_BIG_ENDIAN=0");

    // Set target triple for clang — must exactly match what was used to compile the .a libraries.
    // Vendor is "none" (NDK convention) and the API level suffix controls __ANDROID_API__ guards
    // in NDK headers. Read from ANDROID_API_LEVEL env var, defaulting to 35.
    let api_level = env::var("ANDROID_API_LEVEL").unwrap_or_else(|_| "35".to_string());
    let target_triple = format!("{}-none-linux-android{}", target_arch, api_level);
    builder = builder.clang_arg(format!("--target={}", target_triple));

    // Add Android NDK sysroot if available
    if let Ok(ndk_home) = env::var("ANDROID_NDK_HOME") {
        println!("NDK Home: {}", ndk_home);

        // Modern NDK uses unified headers in toolchains/llvm/prebuilt/{host}/sysroot
        let target_sysroot_dir = if cfg!(target_os = "linux") {
            "linux-x86_64"
        } else if cfg!(target_os = "macos") {
            "darwin-x86_64"
        } else if cfg!(target_os = "windows") {
            "windows-x86_64"
        } else {
            "linux-x86_64" // fallback
        };

        let sysroot = format!(
            "{}/toolchains/llvm/prebuilt/{}/sysroot",
            ndk_home, target_sysroot_dir
        );
        builder = builder.clang_arg(format!("--sysroot={}", sysroot));
    }

    let bindings = builder.generate().expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

fn generate_bindings_default() {
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
}

fn download_and_extract(url: &str) {
    // extract file name from end of url
    let file_name = url.split('/').last().unwrap_or("pre-compiled.zip");

    // download
    let out_dir = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("pjlibs");
    fs::create_dir_all(&out_dir).unwrap();
    let archive_path = out_dir.join(file_name);

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
    let info = get_target_info();
    format!("-{}-{}-{}-{}", info.2, info.1, info.0, real_env())
}

fn create_config() {
    let config_path = Path::new("pjproject/pjlib/include/pj/config_site.h");
    if !config_path.exists() {
        let mut file = File::create(config_path).expect("config_site.h not created, Error!");
        use std::io::Write;
        file.write_all(b"/* Activate Android specific settings in the 'config_site_sample.h' */\n#define PJ_CONFIG_ANDROID 1\n#include <pj/config_site_sample.h>\n\n#define PJMEDIA_HAS_VIDEO 0\n")
            .expect("Failed to write config_site.h");
        println!("config_site.h created");
    } else {
        println!("config_site.h already exists, leaving unchanged");
    }
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
    println!("cargo:rustc-link-lib={}webrtc{}", t, s);
    println!("cargo:rustc-link-lib={}yuv{}", t, s);

    // Dependencies
    println!("cargo:rustc-link-lib=ssl");
    println!("cargo:rustc-link-lib=crypto");
    println!("cargo:rustc-link-lib=z");
    println!("cargo:rustc-link-lib=asound");
    println!("cargo:rustc-link-lib=uuid");
}

// ANDROID
fn link_libs_android() {
    let project_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let info = get_target_info();
    let target_triple = format!("{}-{}-linux-android", info.2, info.1);

    println!("cargo:rustc-link-search={}/pjlibs", project_dir);

    // Core PJSIP libraries
    println!("cargo:rustc-link-lib=static=pjsua-{}", target_triple);
    println!("cargo:rustc-link-lib=static=pjsip-{}", target_triple);
    println!("cargo:rustc-link-lib=static=pjsip-simple-{}", target_triple);
    println!("cargo:rustc-link-lib=static=pjsip-ua-{}", target_triple);

    // Base library
    println!("cargo:rustc-link-lib=static=pj-{}", target_triple);

    // Media libraries
    println!("cargo:rustc-link-lib=static=pjmedia-{}", target_triple);
    println!(
        "cargo:rustc-link-lib=static=pjmedia-codec-{}",
        target_triple
    );
    println!(
        "cargo:rustc-link-lib=static=pjmedia-videodev-{}",
        target_triple
    );
    println!(
        "cargo:rustc-link-lib=static=pjmedia-audiodev-{}",
        target_triple
    );
    println!("cargo:rustc-link-lib=static=pjsdp-{}", target_triple);

    // NAT and utilities
    println!("cargo:rustc-link-lib=static=pjnath-{}", target_triple);
    println!("cargo:rustc-link-lib=static=pjlib-util-{}", target_triple);

    // Codec libraries
    println!("cargo:rustc-link-lib=static=gsmcodec-{}", target_triple);
    println!("cargo:rustc-link-lib=static=resample-{}", target_triple);
    println!("cargo:rustc-link-lib=static=srtp-{}", target_triple);
    println!("cargo:rustc-link-lib=static=speex-{}", target_triple);
    println!("cargo:rustc-link-lib=static=ilbccodec-{}", target_triple);
    println!("cargo:rustc-link-lib=static=g7221codec-{}", target_triple);
    println!("cargo:rustc-link-lib=static=webrtc-{}", target_triple);
    println!("cargo:rustc-link-lib=static=yuv-{}", target_triple);

    // Android system libraries
    println!("cargo:rustc-link-lib=c");
    println!("cargo:rustc-link-lib=m");
    println!("cargo:rustc-link-lib=log");
    println!("cargo:rustc-link-lib=OpenSLES");
    println!("cargo:rustc-link-lib=c++_shared");
    println!("cargo:rustc-link-lib=dl");
    println!("cargo:rustc-link-lib=mediandk");
    println!("cargo:rustc-link-lib=GLESv2");
    println!("cargo:rustc-link-lib=EGL");
    println!("cargo:rustc-link-lib=android");
}
