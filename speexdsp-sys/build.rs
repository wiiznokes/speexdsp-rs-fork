use std::env;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

fn main() {
    let vendored = env::var("CARGO_FEATURE_VENDORED").is_ok();

    let dst = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    let mut include_paths: Vec<PathBuf> = Vec::new();

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

        include_paths =
            libs.get_by_name("speexdsp").unwrap().include_paths.clone();
    } else {
        include_paths.push(PathBuf::from("speexdsp/include"));
        include_paths.push(PathBuf::from("speexdsp/libspeexdsp"));

        // generate speexdsp_config_types.h.in manually to not depend on cmake
        {
            let template = std::fs::read_to_string(
                "speexdsp/include/speex/speexdsp_config_types.h.in",
            )
            .unwrap();

            let content = template
                .replace("@INCLUDE_STDINT@", "#include <stdint.h>")
                .replace("@SIZE16@", "int16_t")
                .replace("@USIZE16@", "uint16_t")
                .replace("@SIZE32@", "int32_t")
                .replace("@USIZE32@", "uint32_t");

            std::fs::write(
                "speexdsp/include/speex/speexdsp_config_types.h",
                content,
            )
            .unwrap();
        }

        let mut cfg = cc::Build::new();

        let common_c_files = [
            "buffer.c",
            "fftwrap.c",
            "filterbank.c",
            "jitter.c",
            "kiss_fft.c",
            "kiss_fftr.c",
            "mdf.c",
            "preprocess.c",
            "resample.c",
            "scal.c",
            "smallft.c",
        ];

        for f in common_c_files {
            cfg.file(Path::new("speexdsp/libspeexdsp").join(f));
        }

        cfg.define("FLOATING_POINT", None);

        // xxx: maybe we should use the .h files in win32 ?
        
        // necessary for windows for some reason
        cfg.define("EXPORT", "");
        // most portable implementation
        cfg.define("USE_SMALLFT", None);

        for path in &include_paths {
            cfg.include(path);
        }

        cfg.out_dir(dst.join("build"));
        cfg.warnings(false);

        cfg.compile("speexdsp");
    }

    for e in ["echo", "jitter", "preprocess", "resampler"].iter() {
        let mut builder = bindgen::builder()
            .size_t_is_usize(true)
            .layout_tests(false)
            .header(format!("data/{}.h", e));

        for header in include_paths.iter() {
            builder =
                builder.clang_arg("-I").clang_arg(header.to_str().unwrap());
        }

        let bindings = builder.generate().unwrap();

        // Manually fix the comment so rustdoc won't try to pick them
        let s = bindings
            .to_string()
            .replace("/**", "/*")
            .replace("/*!", "/*");

        let lib = format!("{}.rs", e);

        let mut file = File::create(dst.join(lib)).unwrap();

        file.write(s.as_bytes()).unwrap();
    }
}
