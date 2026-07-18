use std::io::Write;
use std::path::Path;

use super::{confirm, Ctx};
use crate::api::releases::{CreateRelease, EditRelease};
use crate::cli::ReleaseCmd;
use crate::error::Result;
use crate::out;

pub fn execute(ctx: &Ctx, cmd: ReleaseCmd) -> Result<()> {
    match cmd {
        ReleaseCmd::List { limit } => {
            let repo = ctx.repo()?;
            let items = ctx.client.releases(repo).list(limit.limit)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &items, |w| out::release_table(w, &items))?;
        }
        ReleaseCmd::View { tag } => {
            let repo = ctx.repo()?;
            let release = ctx.client.releases(repo).get_by_tag(&tag)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &release, |w| out::one_release(w, &release))?;
        }
        ReleaseCmd::Create {
            tag,
            name,
            notes,
            target,
            prerelease,
        } => {
            let repo = ctx.repo()?;
            let req = CreateRelease {
                tag: &tag,
                name: name.as_deref(),
                notes: notes.as_deref(),
                target: target.as_deref(),
                prerelease,
            };
            let release = ctx.client.releases(repo).create(&req)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &release, |w| out::one_release(w, &release))?;
        }
        ReleaseCmd::Upload { tag, files } => {
            let repo = ctx.repo()?;
            let releases = ctx.client.releases(repo);
            let mut out = std::io::stdout().lock();
            for file_path in files {
                let asset = releases.upload(&tag, &file_path)?;
                writeln!(out, "{}", asset.name)?;
            }
        }
        ReleaseCmd::Download { tag, dir, pattern } => {
            let repo = ctx.repo()?;
            let release = ctx.client.releases(repo).get_by_tag(&tag)?;
            let assets = release.assets.unwrap_or_default();
            let matching: Vec<_> = assets
                .iter()
                .filter(|a| {
                    pattern
                        .as_deref()
                        .is_none_or(|p| glob_match(p, &a.name))
                })
                .collect();
            let mut out = std::io::stdout().lock();
            if matching.is_empty() {
                if pattern.is_some() {
                    writeln!(out, "No assets match pattern")?;
                } else {
                    writeln!(out, "No release assets")?;
                }
                return Ok(());
            }
            std::fs::create_dir_all(&dir)?;
            for asset in matching {
                let bytes = ctx.client.get_bytes(&asset.browser_download_url)?;
                let path = Path::new(&dir).join(&asset.name);
                std::fs::write(&path, &bytes)?;
                writeln!(out, "Saved {} ({} bytes)", path.display(), bytes.len())?;
            }
        }
        ReleaseCmd::Edit {
            tag,
            name,
            notes,
            prerelease,
        } => {
            let repo = ctx.repo()?;
            let req = EditRelease {
                name: name.as_deref(),
                notes: notes.as_deref(),
                prerelease: prerelease.then_some(true),
            };
            let release = ctx.client.releases(repo).edit(&tag, &req)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &release, |w| out::one_release(w, &release))?;
        }
        ReleaseCmd::Delete { tag, yes } => {
            confirm(&format!("Delete release {tag}"), yes)?;
            let repo = ctx.repo()?;
            ctx.client.releases(repo).delete(&tag)?;
            let mut out = std::io::stdout().lock();
            writeln!(out, "Deleted release {tag}")?;
        }
    }
    Ok(())
}

/// Hand-rolled `*`-wildcard matcher: `*` matches any run (including empty); other
/// bytes must match literally.
pub fn glob_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    glob_at(&p, 0, &n, 0)
}

fn glob_at(p: &[char], pi: usize, n: &[char], ni: usize) -> bool {
    if pi == p.len() {
        return ni == n.len();
    }
    if p[pi] == '*' {
        if pi + 1 == p.len() {
            return true;
        }
        for i in ni..=n.len() {
            if glob_at(p, pi + 1, n, i) {
                return true;
            }
        }
        return false;
    }
    if ni == n.len() {
        return false;
    }
    if p[pi] == '?' {
        return glob_at(p, pi + 1, n, ni + 1);
    }
    if p[pi] != n[ni] {
        return false;
    }
    glob_at(p, pi + 1, n, ni + 1)
}

#[cfg(test)]
mod glob_tests {
    use super::glob_match;

    #[test]
    fn star_matches_empty() {
        assert!(glob_match("*", ""));
        assert!(glob_match("*.txt", "a.txt"));
        assert!(glob_match("pre*post", "prepost"));
    }

    #[test]
    fn star_matches_run() {
        assert!(glob_match("*.tar.xz", "gitee-linux-amd64.tar.xz"));
        assert!(!glob_match("*.tar.xz", "gitee-darwin-arm64.zip"));
    }

    #[test]
    fn literal_must_match() {
        assert!(glob_match("exact", "exact"));
        assert!(!glob_match("exact", "exactx"));
    }

    #[test]
    fn glob_match_question_mark_matches_one_char() {
        assert!(glob_match("v?.txt", "v1.txt"));
        assert!(!glob_match("v?.txt", "v12.txt"));
        assert!(!glob_match("v?.txt", "v.txt"));
        assert!(glob_match("?.txt", "a.txt"));
    }
}
