use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::Config;
use crate::error::{GiteeError, Result};

const PREFIX: &str = "gitee-";

/// Split `PATH` into directories (empty when unset).
pub fn path_dirs() -> Vec<PathBuf> {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default()
}

fn extension_binary_name(name: &str) -> String {
    format!("{PREFIX}{name}")
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.is_file()
        && path
            .metadata()
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

/// Resolve `gitee-{name}` on `PATH`.
pub fn find_on_path(name: &str) -> Option<PathBuf> {
    if name.is_empty() {
        return None;
    }
    let bin = extension_binary_name(name);
    for dir in path_dirs() {
        let path = dir.join(&bin);
        if is_executable(&path) {
            return Some(path);
        }
        #[cfg(windows)]
        for ext in ["exe", "cmd", "bat"] {
            let path = dir.join(format!("{bin}.{ext}"));
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}

fn file_stem(name: &str) -> &str {
    Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(name)
}

/// Scan `PATH` for `gitee-*` executables; return extension names (without prefix).
pub fn list_on_path() -> Vec<String> {
    let mut names = Vec::new();
    for dir in path_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name_os = entry.file_name();
            let Some(file_name) = file_name_os.to_str() else {
                continue;
            };
            let stem = file_stem(file_name);
            let Some(ext_name) = stem.strip_prefix(PREFIX) else {
                continue;
            };
            if ext_name.is_empty() || !is_executable(&path) {
                continue;
            }
            names.push(ext_name.to_string());
        }
    }
    names.sort();
    names.dedup();
    names
}

/// Whether to inject `GITEE_HOST` for an extension child process.
fn should_set_gitee_host() -> bool {
    std::env::var_os("GITEE_HOST").is_none()
}

/// Exec a `gitee-{name}` binary with `args`, forwarding `GITEE_*` env.
pub fn exec(name: &str, args: &[OsString], host: &str) -> Result<()> {
    let path = find_on_path(name).ok_or_else(|| {
        GiteeError::Usage(format!(
            "extension command '{name}' not found on PATH (expected {PREFIX}{name})"
        ))
    })?;
    let mut cmd = Command::new(&path);
    cmd.args(args);
    if should_set_gitee_host() {
        cmd.env("GITEE_HOST", host);
    }
    if std::env::var("GITEE_TOKEN").is_err() {
        if let Ok(token) = Config::token(host) {
            cmd.env("GITEE_TOKEN", token);
        }
    }
    let status = cmd.status()?;
    std::process::exit(status.code().unwrap_or(1));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    fn temp_bin_dir() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "gitee-ext-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).expect("mkdir");
        path
    }

    #[cfg(unix)]
    fn write_fake_ext(dir: &Path, name: &str) {
        let path = dir.join(format!("{PREFIX}{name}"));
        fs::write(&path, b"#!/bin/sh\n").expect("write");
        let mut perms = fs::metadata(&path).expect("meta").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).expect("chmod");
    }

    #[cfg(not(unix))]
    fn write_fake_ext(dir: &Path, name: &str) {
        let path = dir.join(format!("{PREFIX}{name}"));
        fs::write(&path, b"").expect("write");
    }

    #[test]
    fn should_set_gitee_host_respects_existing_env() {
        let prev = std::env::var_os("GITEE_HOST");
        std::env::set_var("GITEE_HOST", "self.gitee.test");
        assert!(!should_set_gitee_host());
        if let Some(v) = prev {
            std::env::set_var("GITEE_HOST", v);
        } else {
            std::env::remove_var("GITEE_HOST");
        }
        assert!(should_set_gitee_host());
    }

    #[cfg(unix)]
    #[test]
    fn find_on_path_discovers_executable() {
        let dir = temp_bin_dir();
        write_fake_ext(&dir, "foo");
        let prev = std::env::var_os("PATH");
        std::env::set_var("PATH", &dir);
        let found = find_on_path("foo");
        if let Some(p) = prev {
            std::env::set_var("PATH", p);
        } else {
            std::env::remove_var("PATH");
        }
        assert_eq!(found, Some(dir.join("gitee-foo")));
    }

    #[cfg(unix)]
    #[test]
    fn find_on_path_missing_returns_none() {
        let dir = temp_bin_dir();
        let prev = std::env::var_os("PATH");
        std::env::set_var("PATH", &dir);
        let found = find_on_path("nope");
        if let Some(p) = prev {
            std::env::set_var("PATH", p);
        } else {
            std::env::remove_var("PATH");
        }
        assert!(found.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn list_on_path_collects_names() {
        let dir = temp_bin_dir();
        write_fake_ext(&dir, "alpha");
        write_fake_ext(&dir, "beta");
        let prev = std::env::var_os("PATH");
        std::env::set_var("PATH", &dir);
        let names = list_on_path();
        if let Some(p) = prev {
            std::env::set_var("PATH", p);
        } else {
            std::env::remove_var("PATH");
        }
        assert_eq!(names, vec!["alpha".to_string(), "beta".to_string()]);
    }
}
