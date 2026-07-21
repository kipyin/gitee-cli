use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cli::BuildKind;
use crate::config::Config;
use crate::error::{GiteeError, Result};

const PREFIX: &str = "gitee-";

#[cfg(test)]
static TEST_MANAGED_DIR: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

#[cfg(test)]
pub fn set_test_managed_dir(dir: Option<PathBuf>) {
    *TEST_MANAGED_DIR.lock().unwrap_or_else(|e| e.into_inner()) = dir;
}

#[cfg(test)]
pub fn test_managed_env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Split `PATH` into directories (empty when unset).
pub fn path_dirs() -> Vec<PathBuf> {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default()
}

/// Managed extensions live under `<data_dir>/gitee/extensions`. Matches `gh`'s
/// choice of `dirs::data_dir()` (NOT `data_local_dir`).
pub fn managed_dir() -> Result<PathBuf> {
    #[cfg(test)]
    if let Some(p) = TEST_MANAGED_DIR.lock().unwrap_or_else(|e| e.into_inner()).clone() {
        return Ok(p);
    }
    let base = dirs::data_dir().ok_or_else(|| {
        GiteeError::Usage("no data directory available for managed extensions".into())
    })?;
    Ok(base.join("gitee").join("extensions"))
}

/// Directories scanned for `gitee-*` executables (the flat `PATH` portion only;
/// the managed dir is consulted via [`find_installed`] because its layout is
/// `<name>/gitee-<name>`, not a flat bin dir).
fn flat_search_dirs() -> Result<Vec<PathBuf>> {
    Ok(path_dirs())
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

/// Resolve `gitee-{name}` on `PATH` (managed dir is consulted first via
/// [`find_installed`], so an installed extension shadows a same-named binary
/// elsewhere on `PATH`).
pub fn find_on_path(name: &str) -> Option<PathBuf> {
    if name.is_empty() {
        return None;
    }
    if let Ok(Some(p)) = find_installed(name) {
        return Some(p);
    }
    let bin = extension_binary_name(name);
    for dir in flat_search_dirs().ok()? {
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

/// Scan `PATH` and the managed dir for `gitee-*` executables; return extension
/// names (without prefix), sorted and deduped.
pub fn list_on_path() -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(installed) = list_installed() {
        names.extend(installed);
    }
    for dir in flat_search_dirs().unwrap_or_default() {
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

/// Names of extensions installed in the managed dir (subdirs containing a
/// `gitee-<name>` executable at the root).
pub fn list_installed() -> Result<Vec<String>> {
    let dir = managed_dir()?;
    let mut names = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(names);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if find_installed(name)?.is_some() {
            names.push(name.to_string());
        }
    }
    names.sort();
    Ok(names)
}

/// Path to `<managed>/<name>/gitee-<name>` if present (and executable on Unix).
pub fn find_installed(name: &str) -> Result<Option<PathBuf>> {
    if name.is_empty() {
        return Ok(None);
    }
    let dir = managed_dir()?.join(name);
    let bin = dir.join(extension_binary_name(name));
    if is_executable(&bin) {
        return Ok(Some(bin));
    }
    #[cfg(windows)]
    {
        for ext in ["exe", "cmd", "bat"] {
            let p = dir.join(format!("{}-{name}.{ext}", PREFIX));
            if p.is_file() {
                return Ok(Some(p));
            }
        }
    }
    Ok(None)
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

fn parse_build_kind(s: Option<&str>) -> Result<Option<BuildKind>> {
    match s {
        None => Ok(None),
        Some("cargo") => Ok(Some(BuildKind::Cargo)),
        Some("npm") => Ok(Some(BuildKind::Npm)),
        Some(other) => Err(GiteeError::Usage(format!(
            "unknown --build value '{other}'; expected 'cargo' or 'npm'"
        ))),
    }
}

/// `owner/repo` → extension name (the repo's last path segment, stripped of an
/// optional `gitee-` prefix). Accepts full URLs too.
fn repo_to_name(repo: &str) -> Result<String> {
    let trimmed = repo.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(GiteeError::Usage("install: repo argument is empty".into()));
    }
    if !trimmed.contains('/') && !trimmed.contains(':') {
        return Err(GiteeError::Usage(format!(
            "install: repo '{repo}' must be owner/name or a full URL"
        )));
    }
    let last = trimmed.rsplit(['/', ':']).next().unwrap_or(trimmed);
    if last.is_empty() {
        return Err(GiteeError::Usage(format!(
            "install: could not derive extension name from '{repo}'"
        )));
    }
    let name = last.strip_prefix(PREFIX).unwrap_or(last);
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
        || name == "."
        || name == ".."
    {
        return Err(GiteeError::Usage(format!(
            "install: invalid extension name '{name}' derived from '{repo}'"
        )));
    }
    Ok(name.to_string())
}

fn clone_url(repo: &str, host: &str) -> String {
    if repo.starts_with("http://")
        || repo.starts_with("https://")
        || repo.starts_with("git@")
        || repo.starts_with("ssh://")
    {
        return repo.to_string();
    }
    let owner_repo = repo.trim_start_matches('/');
    format!("https://{host}/{owner_repo}")
}

fn last_commit_short_sha(dir: &Path) -> Option<String> {
    let out = Command::new("git")
        .args(["-C"])
        .arg(dir)
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn run_git(dir: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new("git")
        .current_dir(dir)
        .args(args)
        .status()
        .map_err(|e| GiteeError::Usage(format!("git: {e}")))?;
    if !status.success() {
        return Err(GiteeError::Usage(format!(
            "git {} failed (exit {:?})",
            args.join(" "),
            status.code()
        )));
    }
    Ok(())
}

fn build_cargo(ext_dir: &Path, name: &str) -> Result<()> {
    let status = Command::new("cargo")
        .current_dir(ext_dir)
        .args(["build", "--release"])
        .status()
        .map_err(|e| GiteeError::Usage(format!("cargo: {e}")))?;
    if !status.success() {
        return Err(GiteeError::Usage(format!(
            "cargo build --release failed (exit {:?})",
            status.code()
        )));
    }
    let target = ext_dir.join("target/release");
    let bin_name = crate_name(ext_dir).unwrap_or_else(|| extension_binary_name(name));
    let src = target.join(&bin_name);
    let dst = ext_dir.join(extension_binary_name(name));
    if !src.exists() {
        return Err(GiteeError::Usage(format!(
            "cargo build produced no binary at {}",
            src.display()
        )));
    }
    std::fs::copy(&src, &dst).map_err(|e| GiteeError::Usage(format!("copy: {e}")))?;
    make_executable(&dst);
    Ok(())
}

fn crate_name(ext_dir: &Path) -> Option<String> {
    let toml = std::fs::read_to_string(ext_dir.join("Cargo.toml")).ok()?;
    for line in toml.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("name") {
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix('=') {
                let v = rest.trim().trim_matches('"').trim_matches('\'');
                if !v.is_empty() {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

fn build_npm(ext_dir: &Path) -> Result<()> {
    let has_build = std::fs::read_to_string(ext_dir.join("package.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| {
            v.get("scripts")
                .and_then(|s| s.get("build"))
                .map(|b| b.is_string())
        })
        .unwrap_or(false);
    let install_status = Command::new("npm")
        .current_dir(ext_dir)
        .arg("install")
        .status()
        .map_err(|e| GiteeError::Usage(format!("npm install: {e}")))?;
    if !install_status.success() {
        return Err(GiteeError::Usage(format!(
            "npm install failed (exit {:?})",
            install_status.code()
        )));
    }
    if has_build {
        let build_status = Command::new("npm")
            .current_dir(ext_dir)
            .args(["run", "build"])
            .status()
            .map_err(|e| GiteeError::Usage(format!("npm run build: {e}")))?;
        if !build_status.success() {
            return Err(GiteeError::Usage(format!(
                "npm run build failed (exit {:?})",
                build_status.code()
            )));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755));
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {}

/// Install `owner/repo` into the managed extensions dir. `git` is required.
pub fn install(repo: &str, build: Option<&str>, yes: bool, host: &str) -> Result<()> {
    let name = repo_to_name(repo)?;
    let build = parse_build_kind(build)?;
    let dir = managed_dir()?;
    std::fs::create_dir_all(&dir).map_err(|e| GiteeError::Usage(e.to_string()))?;
    let ext_dir = dir.join(&name);
    if ext_dir.exists() {
        return Err(GiteeError::Usage(format!(
            "extension '{name}' already installed at {}",
            ext_dir.display()
        )));
    }
    let url = clone_url(repo, host);
    let action = format!(
        "install extension '{name}' from {url} — you are about to download and run arbitrary code"
    );
    crate::cmd::confirm(&action, yes)?;
    run_git(&dir, &["clone", "--depth", "1", &url, &name])?;
    if let Some(sha) = last_commit_short_sha(&ext_dir) {
        eprintln!("cloned {url} @ {sha}");
    }
    match build {
        Some(BuildKind::Cargo) => build_cargo(&ext_dir, &name)?,
        Some(BuildKind::Npm) => build_npm(&ext_dir)?,
        None => {}
    }
    if find_installed(&name)?.is_none() {
        return Err(GiteeError::Usage(format!(
            "install: repo '{repo}' did not produce a {PREFIX}{name} executable at {}",
            ext_dir.display()
        )));
    }
    println!("installed extension '{name}'");
    Ok(())
}

/// Scaffold a new extension project in `cwd` (not the managed dir).
pub fn create(name: &str, cargo: bool) -> Result<()> {
    validate_ext_name(name)?;
    let cwd = std::env::current_dir().map_err(|e| GiteeError::Usage(e.to_string()))?;
    let bin = extension_binary_name(name);
    if cargo {
        let toml = format!(
            "[package]\nname = \"{bin}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nclap = {{ version = \"4\", features = [\"derive\"] }}\n"
        );
        std::fs::write(cwd.join("Cargo.toml"), toml)
            .map_err(|e| GiteeError::Usage(e.to_string()))?;
        let src_dir = cwd.join("src");
        std::fs::create_dir_all(&src_dir).map_err(|e| GiteeError::Usage(e.to_string()))?;
        let main = format!(
            "use clap::Parser;\n\n#[derive(Parser)]\n#[command(name = \"{bin}\", about = \"demo extension\")]\nstruct Cli {{\n    args: Vec<String>,\n}}\n\nfn main() {{\n    let _cli = Cli::parse();\n    println!(\"demo extension\");\n}}\n"
        );
        std::fs::write(src_dir.join("main.rs"), main)
            .map_err(|e| GiteeError::Usage(e.to_string()))?;
    } else {
        #[cfg(unix)]
        {
            let script = "#!/bin/sh\necho \"demo extension\"\n";
            let path = cwd.join(&bin);
            std::fs::write(&path, script).map_err(|e| GiteeError::Usage(e.to_string()))?;
            make_executable(&path);
        }
        #[cfg(not(unix))]
        {
            let path = cwd.join(format!("{bin}.cmd"));
            let script = "@echo off\necho demo extension\n";
            std::fs::write(&path, script).map_err(|e| GiteeError::Usage(e.to_string()))?;
        }
    }
    let readme = format!(
        "# {bin}\n\nA `gitee` extension. Invoked as `gitee {name} ...`.\n\n## Environment contract\n\nThe `gitee` CLI forwards these to every extension child process:\n\n- `GITEE_TOKEN` — the active personal access token (or your own `$GITEE_TOKEN` if set).\n- `GITEE_HOST` — the active Gitee host (e.g. `gitee.com`), unless you already exported it.\n- All trailing argv, forwarded verbatim.\n\n## Build\n\n{build}\n\n## Install\n\n```\ngitee extension install <owner/{name}>\n```\n",
        build = if cargo { "```cargo build --release``` produces a `gitee-{name}` binary at `target/release/`." } else { "No build step; the `gitee-{name}` script at the repo root is the entry point." }
    );
    std::fs::write(cwd.join("README.md"), readme)
        .map_err(|e| GiteeError::Usage(e.to_string()))?;
    println!("scaffolded extension '{name}' in {}", cwd.display());
    Ok(())
}

fn validate_ext_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.starts_with('-')
        || name.contains('/')
        || name.contains('\\')
        || name.contains(' ')
        || name.contains('\0')
        || name == "."
        || name == ".."
    {
        return Err(GiteeError::Usage(format!(
            "invalid extension name: '{name}'"
        )));
    }
    Ok(())
}

/// Remove an installed extension from the managed dir.
pub fn remove(name: &str, yes: bool) -> Result<()> {
    validate_ext_name(name)?;
    let dir = managed_dir()?.join(name);
    if !dir.exists() {
        return Err(GiteeError::Usage(format!(
            "extension '{name}' is not installed (no such dir: {})",
            dir.display()
        )));
    }
    let action = format!(
        "remove extension '{name}' at {}",
        dir.display()
    );
    crate::cmd::confirm(&action, yes)?;
    std::fs::remove_dir_all(&dir).map_err(|e| GiteeError::Usage(e.to_string()))?;
    println!("removed extension '{name}'");
    Ok(())
}

/// Pull (and rebuild, if a build marker is present) one or all installed extensions.
pub fn upgrade(name: Option<&str>) -> Result<()> {
    let dir = managed_dir()?;
    let names: Vec<String> = match name {
        Some(n) => {
            validate_ext_name(n)?;
            vec![n.to_string()]
        }
        None => list_installed()?,
    };
    if names.is_empty() {
        println!("no installed extensions to upgrade");
        return Ok(());
    }
    for n in names {
        let ext_dir = dir.join(&n);
        if !ext_dir.is_dir() {
            return Err(GiteeError::Usage(format!(
                "extension '{n}' is not installed at {}",
                ext_dir.display()
            )));
        }
        run_git(&ext_dir, &["pull", "--ff-only"])?;
        let needs_cargo = ext_dir.join("Cargo.toml").exists();
        let needs_npm = ext_dir.join("package.json").exists();
        if needs_cargo {
            build_cargo(&ext_dir, &n)?;
        } else if needs_npm {
            build_npm(&ext_dir)?;
        }
        println!("upgraded extension '{n}'");
    }
    Ok(())
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
        let _lock = test_managed_env_lock();
        let managed = unique_managed_dir();
        set_test_managed_dir(Some(managed.clone()));
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
        set_test_managed_dir(None);
        let _ = fs::remove_dir_all(&managed);
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(found, Some(dir.join("gitee-foo")));
    }

    #[cfg(unix)]
    #[test]
    fn find_on_path_missing_returns_none() {
        let _lock = test_managed_env_lock();
        let managed = unique_managed_dir();
        set_test_managed_dir(Some(managed.clone()));
        let dir = temp_bin_dir();
        let prev = std::env::var_os("PATH");
        std::env::set_var("PATH", &dir);
        let found = find_on_path("nope");
        if let Some(p) = prev {
            std::env::set_var("PATH", p);
        } else {
            std::env::remove_var("PATH");
        }
        set_test_managed_dir(None);
        let _ = fs::remove_dir_all(&managed);
        let _ = fs::remove_dir_all(&dir);
        assert!(found.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn list_on_path_collects_names() {
        let _lock = test_managed_env_lock();
        let managed = unique_managed_dir();
        set_test_managed_dir(Some(managed.clone()));
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
        set_test_managed_dir(None);
        let _ = fs::remove_dir_all(&managed);
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(names, vec!["alpha".to_string(), "beta".to_string()]);
    }

    fn unique_managed_dir() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "gitee-ext-managed-{}-{}",
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
    fn write_managed_ext(root: &Path, name: &str) {
        let dir = root.join(name);
        fs::create_dir_all(&dir).expect("mkdir");
        write_fake_ext(&dir, name);
    }

    #[cfg(unix)]
    #[test]
    fn list_installed_finds_managed_extensions() {
        let _lock = test_managed_env_lock();
        let dir = unique_managed_dir();
        set_test_managed_dir(Some(dir.clone()));
        write_managed_ext(&dir, "alpha");
        write_managed_ext(&dir, "beta");
        let names = list_installed().expect("list");
        set_test_managed_dir(None);
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(names, vec!["alpha".to_string(), "beta".to_string()]);
    }

    #[cfg(unix)]
    #[test]
    fn find_installed_resolves_managed_binary() {
        let _lock = test_managed_env_lock();
        let dir = unique_managed_dir();
        set_test_managed_dir(Some(dir.clone()));
        write_managed_ext(&dir, "foo");
        let found = find_installed("foo").expect("find").expect("present");
        set_test_managed_dir(None);
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(found.file_name().unwrap(), "gitee-foo");
    }

    #[test]
    fn find_installed_missing_returns_none() {
        let _lock = test_managed_env_lock();
        let dir = unique_managed_dir();
        set_test_managed_dir(Some(dir.clone()));
        let found = find_installed("nope").expect("find");
        set_test_managed_dir(None);
        let _ = fs::remove_dir_all(&dir);
        assert!(found.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn managed_dir_shadows_path_in_find_on_path() {
        let _lock = test_managed_env_lock();
        let managed = unique_managed_dir();
        set_test_managed_dir(Some(managed.clone()));
        write_managed_ext(&managed, "dup");

        let path_dir = temp_bin_dir();
        write_fake_ext(&path_dir, "dup");
        let prev = std::env::var_os("PATH");
        std::env::set_var("PATH", &path_dir);

        let found = find_on_path("dup").expect("found");

        if let Some(p) = prev {
            std::env::set_var("PATH", p);
        } else {
            std::env::remove_var("PATH");
        }
        set_test_managed_dir(None);
        let _ = fs::remove_dir_all(&managed);
        let _ = fs::remove_dir_all(&path_dir);

        assert!(found.starts_with(&managed), "managed dir should win: {found:?}");
    }

    #[test]
    fn repo_to_name_strips_gitee_prefix() {
        assert_eq!(repo_to_name("owner/my-ext").unwrap(), "my-ext");
        assert_eq!(repo_to_name("owner/gitee-my-ext").unwrap(), "my-ext");
        assert_eq!(
            repo_to_name("https://gitee.com/owner/gitee-foo").unwrap(),
            "foo"
        );
        assert!(repo_to_name("").is_err());
        assert!(repo_to_name("owner/").is_err());
        assert!(repo_to_name("just-a-name").is_err());
        assert!(repo_to_name("owner/../").is_err());
    }

    #[test]
    fn parse_build_kind_accepts_known_values() {
        assert!(matches!(parse_build_kind(None).unwrap(), None));
        assert!(matches!(parse_build_kind(Some("cargo")).unwrap(), Some(BuildKind::Cargo)));
        assert!(matches!(parse_build_kind(Some("npm")).unwrap(), Some(BuildKind::Npm)));
        assert!(parse_build_kind(Some("go")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn create_scaffolds_shell_script() {
        let dir = unique_managed_dir();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();
        create("demo", false).expect("create");
        std::env::set_current_dir(&prev).unwrap();
        let script = dir.join("gitee-demo");
        assert!(script.is_file(), "script should exist at {}", script.display());
        let body = fs::read_to_string(&script).unwrap();
        assert!(body.contains("demo extension"));
        let readme = fs::read_to_string(dir.join("README.md")).unwrap();
        assert!(readme.contains("GITEE_TOKEN"));
        assert!(readme.contains("GITEE_HOST"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn create_scaffolds_cargo_project() {
        let dir = unique_managed_dir();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();
        create("demo", true).expect("create");
        std::env::set_current_dir(&prev).unwrap();
        assert!(dir.join("Cargo.toml").is_file());
        assert!(dir.join("src/main.rs").is_file());
        let toml = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
        assert!(toml.contains("gitee-demo"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn remove_errors_when_not_installed() {
        let _lock = test_managed_env_lock();
        let dir = unique_managed_dir();
        set_test_managed_dir(Some(dir.clone()));
        let err = remove("nope", true).unwrap_err();
        set_test_managed_dir(None);
        let _ = fs::remove_dir_all(&dir);
        assert!(err.to_string().contains("not installed"));
    }

    #[cfg(unix)]
    #[test]
    fn install_errors_on_already_installed() {
        let _lock = test_managed_env_lock();
        let dir = unique_managed_dir();
        set_test_managed_dir(Some(dir.clone()));
        write_managed_ext(&dir, "foo");
        let err = install("owner/foo", None, true, "gitee.com").unwrap_err();
        set_test_managed_dir(None);
        let _ = fs::remove_dir_all(&dir);
        assert!(err.to_string().contains("already installed"));
    }
}
