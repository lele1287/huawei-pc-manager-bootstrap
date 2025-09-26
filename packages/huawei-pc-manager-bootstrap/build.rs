use std::env;
use std::fs;
use std::path::{Path, PathBuf};

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

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_else(|_| "windows".to_string());
    let (prefix, exts) = match target_os.as_str() {
        "windows" => ("version", vec![".dll"]),
        "macos" => ("libversion", vec![".dylib"]),
        _ => ("libversion", vec![".so"]),
    };

    let profile = env::var("PROFILE").unwrap_or_else(|_| "release".to_string());

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    // workspace root is two levels up from packages/huawei-pc-manager-bootstrap
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| manifest_dir.clone());

    let mut candidates = Vec::new();
    candidates.push(workspace_root.join("target").join(&profile));
    candidates.push(workspace_root.join("target").join(&profile).join("deps"));
    // Also check the version package target/ dir (in case the project isn't a workspace)
    candidates.push(manifest_dir.join("..").join("version").join("target").join(&profile));
    candidates.push(manifest_dir.join("..").join("version").join("target").join(&profile).join("deps"));

    let mut found: Option<PathBuf> = None;
    for cand in candidates {
        if let Some(p) = find_cdylib_in(&cand, prefix, &exts) {
            found = Some(p);
            break;
        }
    }

    if found.is_none() {
        println!("cargo:warning=Could not locate version cdylib in workspace target directories. Searched candidates.");
        return;
    }

    let candidate = found.unwrap();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let dst = out_dir.join(candidate.file_name().unwrap());
    if let Err(e) = fs::copy(&candidate, &dst) {
        println!("cargo:warning=Failed to copy cdylib from {:?} to {:?}: {}", candidate, dst, e);
        return;
    }

    let dst_str = dst.to_string_lossy();
    println!("cargo:rustc-env=CARGO_CDYLIB_FILE_VERSION_version={}", dst_str);
}
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

fn get_git_version() -> String {
    let version = env::var("CARGO_PKG_VERSION").unwrap();

    let child = Command::new("git").args(["describe", "--always"]).output();
    match child {
        Ok(child) => {
            version
                + "-"
                + String::from_utf8(child.stdout)
                    .expect("failed to read stdout")
                    .as_str()
        }
        Err(_) => version,
    }
}

fn main() {
    let version = get_git_version();
    let mut version_file =
        File::create(Path::new(&env::var("OUT_DIR").unwrap()).join("VERSION")).unwrap();
    version_file.write_all(version.trim().as_bytes()).unwrap();
}
