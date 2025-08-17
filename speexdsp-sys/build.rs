extern crate autotools;
extern crate bindgen;
extern crate system_deps;

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{env, fs};

fn format_write(builder: bindgen::Builder) -> String {
    builder
        .generate()
        .unwrap()
        .to_string()
        .replace("/**", "/*")
        .replace("/*!", "/*")
}

fn main() {
    let vendored = env::var("CARGO_FEATURE_VENDORED").is_ok();

    let mut headers: Vec<PathBuf> = Vec::new();

    if !vendored {
        let libs = system_deps::Config::new()
            .add_build_internal("speexdsp", |lib, version| {
                // TODO: decide how to fetch the source
                let dst = autotools::build("speexdsp");
                system_deps::Library::from_internal_pkg_config(
                    dst, lib, version,
                )
            })
            .probe()
            .unwrap();

        headers = libs.get_by_name("speexdsp").unwrap().include_paths.clone();
    } else {
        let dst = PathBuf::from(env::var_os("OUT_DIR").unwrap());

        let include = Path::new("speexdsp/include");

        add_h_files(&mut headers, &include);

        let mut cfg = cc::Build::new();

        add_c_files(&mut cfg, "speexdsp/libspeexdsp");

        cfg.define("FLOATING_POINT", None).define("EXPORT", "");
        cfg.define("USE_SMALLFT", None);

        cfg.include(include);
        cfg.out_dir(dst.join("lib"));
        cfg.warnings(false);

        cfg.compile("speexdsp");
    }

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    for e in ["echo", "jitter", "preprocess", "resampler"].iter() {
        let mut builder = bindgen::builder()
            .size_t_is_usize(true)
            .layout_tests(false)
            .header(format!("data/{}.h", e));

        for header in headers.iter() {
            builder =
                builder.clang_arg("-I").clang_arg(header.to_str().unwrap());
        }

        // Manually fix the comment so rustdoc won't try to pick them
        let s = format_write(builder);

        let lib = format!("{}.rs", e);

        let mut file = File::create(out_path.join(lib)).unwrap();

        let _ = file.write(s.as_bytes());
    }
}

fn add_c_files(build: &mut cc::Build, path: impl AsRef<Path>) {
    let path = path.as_ref();
    if !path.exists() {
        panic!("Path {} does not exist", path.display());
    }
    // sort the C files to ensure a deterministic build for reproducible builds
    let dir = path.read_dir().unwrap();
    let mut paths = dir.collect::<std::io::Result<Vec<_>>>().unwrap();
    paths.sort_by_key(|e| e.path());

    for e in paths {
        let path = e.path();
        if e.file_type().unwrap().is_dir() {
            // skip dirs for now
        } else if path.extension().and_then(|s| s.to_str()) == Some("c") {
            build.file(&path);
        }
    }
}

fn add_h_files(headers: &mut Vec<PathBuf>, path: &Path) {
    for e in path.read_dir().unwrap() {
        let e = e.unwrap();
        let path = e.path();

        if e.file_type().unwrap().is_dir() {
            add_h_files(headers, &path)
        } else if path.extension().and_then(|s| s.to_str()) == Some("h") {
            headers.push(path);
        }
    }
}
