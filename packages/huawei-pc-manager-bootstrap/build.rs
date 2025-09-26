use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

fn find_cdylib_in(dir: &Path, prefix: &str, exts: &[&str]) -> Option<PathBuf> {
    if !dir.exists() {
        return None;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                for ext in exts {
                    if name.starts_with(prefix) && name.ends_with(ext) {
                        return Some(p);
                    }
                }
            }
        }
    }
    None
}

fn get_git_version() -> String {
    let version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string());
    match Command::new("git").args(["describe", "--always"]).output() {
        Ok(child) => {
            let out = String::from_utf8_lossy(&child.stdout).trim().to_string();
            if out.is_empty() {
                version
            } else {
                format!("{}-{}", version, out)
            }
        }
        Err(_) => version,
    }
}

fn write_version_file(out_dir: &Path) {
    let version = get_git_version();
    let version_path = out_dir.join("VERSION");
    if let Ok(mut file) = fs::File::create(&version_path) {
        let _ = file.write_all(version.trim().as_bytes());
    } else {
        println!("cargo:warning=Failed to write VERSION to {:?}", version_path);
    }
}

fn main() {
    // 1) write VERSION
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    write_version_file(&out_dir);

    // 2) try to locate and copy cdylib produced by `version` crate, and export env var
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_else(|_| "windows".to_string());
    let (prefix, exts) = match target_os.as_str() {
        "windows" => ("version", vec![".dll"]),
        "macos" => ("libversion", vec![".dylib"]),
        _ => ("libversion", vec![".so"]),
    };

    let profile = env::var("PROFILE").unwrap_or_else(|_| "release".to_string());

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| manifest_dir.clone());

    let mut candidates = Vec::new();
    candidates.push(workspace_root.join("target").join(&profile));
    candidates.push(workspace_root.join("target").join(&profile).join("deps"));
    candidates.push(manifest_dir.join("..").join("version").join("target").join(&profile));
    candidates.push(manifest_dir.join("..").join("version").join("target").join(&profile).join("deps"));

    let mut found: Option<PathBuf> = None;
    for cand in candidates {
        if let Some(p) = find_cdylib_in(&cand, prefix, &exts) {
            found = Some(p);
            break;
        }
    }

    if let Some(candidate) = found {
        let dst = out_dir.join(candidate.file_name().unwrap());
        if let Err(e) = fs::copy(&candidate, &dst) {
            println!("cargo:warning=Failed to copy cdylib from {:?} to {:?}: {}", candidate, dst, e);
        } else {
            let dst_str = dst.to_string_lossy();
            println!("cargo:rustc-env=CARGO_CDYLIB_FILE_VERSION_version={}", dst_str);
        }
    } else {
        println!("cargo:warning=Could not locate version cdylib in workspace target directories.");
    }
}
