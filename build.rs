extern crate bindgen;
extern crate cmake;
extern crate pkg_config;

use cmake::Config;
use std::env;

fn main() {
    let include_paths = match (
        pkg_config::Config::new()
            .atleast_version("5.0")
            .probe("assimp"),
        pkg_config::probe_library("IrrXML"),
    ) {
        (Ok(assimp), Ok(irrxml)) => {
            for path in assimp.link_paths {
                println!("cargo:rustc-link-path={}", path.to_str().unwrap());
            }
            for lib in assimp.libs {
                println!("cargo:rustc-link-lib={}", lib);
            }

            for path in irrxml.link_paths {
                println!("cargo:rustc-link-path={}", path.to_str().unwrap());
            }
            for lib in irrxml.libs {
                println!("cargo:rustc-link-lib={}", lib);
            }

            assimp
                .include_paths
                .into_iter()
                .chain(irrxml.include_paths)
                .map(|p| p.into_os_string().into_string().unwrap())
                .collect::<Vec<_>>()
        }
        _ => {
            // Compile assimp from source.
            // Disable unnecessary stuff, it takes long enough to compile already.
            let dst = Config::new("assimp")
                .define("ASSIMP_BUILD_ASSIMP_TOOLS", "OFF")
                .define("ASSIMP_BUILD_TESTS", "OFF")
                .define("ASSIMP_INSTALL_PDB", "OFF")
                .define("BUILD_SHARED_LIBS", "OFF")
                .define("DASSIMP_BUILD_ZLIB", "ON")
                .define("LIBRARY_SUFFIX", "")
                .define("CMAKE_SUPPRESS_DEVELOPER_WARNINGS", "ON")
                .define("CMAKE_C_COMPILER", "clang")
                .define("DCMAKE_CXX_COMPILER", "clang++")
                // For some reason, using `.pic(true)` doesn't work here, only
                // specifically setting it in CFLAGS.
                //
                // Also, we have to be really explicit with `-Wno-error=..` since Assimp
                // adds our CFLAGS _after_ theirs, with no way of disabling it (despite
                // what their `Build.md` might tell you. When will C/C++ devs stop setting
                // `-Werror` without a way to disable it.
                .define("CMAKE_C_FLAGS", "-fPIC -Wno-error=all -Wno-error=pedantic")
                .define("CMAKE_CXX_FLAGS", "-fPIC -Wno-error=all -Wno-error=pedantic")
                .uses_cxx11()
                .build();

            let dst = dst.join("lib");
            println!("cargo:rustc-link-search=native={}", dst.display());

            // There's no way to extract this from `cmake::Config` so we have to emulate their
            // behaviour here (see the source for `cmake::Config::build`).
            let debug_postfix = match (
                &env::var("OPT_LEVEL").unwrap_or_default()[..],
                &env::var("PROFILE").unwrap_or_default()[..],
            ) {
                ("1", _) | ("2", _) | ("3", _) | ("s", _) | ("z", _) => "",
                ("0", _) => "d",
                (_, "debug") => "d",
                (_, _) => "",
            };

            println!("cargo:rustc-link-lib=static=assimp{}", debug_postfix);
            println!("cargo:rustc-link-lib=static=IrrXML{}", debug_postfix);

            vec![
                "assimp/include".to_string(),
                "assimp/contrib/irrXML".to_string(),
            ]
        }
    };

    // Link to libstdc++ on GNU.
    let target = env::var("TARGET").unwrap();
    if target.contains("gnu") {
        println!("cargo:rustc-link-lib=stdc++");
    } else if target.contains("apple") {
        println!("cargo:rustc-link-lib=c++");
    }

    println!("cargo:rerun-if-changed=wrapper.h");

    let mut bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .whitelist_type("ai.*")
        .whitelist_function("ai.*")
        .whitelist_var("ai.*")
        .whitelist_var("AI_.*")
        .derive_partialeq(true)
        .derive_eq(true)
        .derive_hash(true)
        .derive_debug(true);

    for path in include_paths {
        bindings = bindings.clang_args(&["-F", &path]);
    }

    let out_path = std::path::PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let bindings = bindings
        .clang_args(&["-F", out_path.join("include").to_str().unwrap()])
        .generate()
        .expect("Unable to generate bindings");

    let bindings_path = out_path.join("bindings.rs");
    bindings
        .write_to_file(&bindings_path)
        .expect("Couldn't write bindings");

    println!("cargo:rerun-if-changed=build.rs");
}
