use std::io::{Read, Write};

use super::{confirm, Ctx};
use crate::api::gists::{CreateGist, UpdateGist};
use crate::cli::GistCmd;
use crate::error::{GiteeError, Result};
use crate::out;

pub fn execute(ctx: &Ctx, cmd: GistCmd) -> Result<()> {
    match cmd {
        GistCmd::List { limit } => {
            let items = ctx.client.gists().list(limit.limit)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &items, |w| out::gist_table(w, &items))?;
        }
        GistCmd::View { id, raw } => {
            let gist = ctx.client.gists().get(&id)?;
            let mut out = std::io::stdout().lock();
            if raw && ctx.out.json.is_none() {
                out::gist_raw(&mut out, &gist)?;
            } else {
                ctx.out
                    .render(&mut out, &gist, |w| out::one_gist(w, &gist))?;
            }
        }
        GistCmd::Create {
            files,
            desc,
            public,
            filename,
        } => {
            let pairs = read_gist_files(&files, filename.as_deref())?;
            let description = desc
                .as_deref()
                .unwrap_or(&pairs[0].0)
                .to_string();
            let description = truncate_description(&description);
            let gist = ctx.client.gists().create(&CreateGist {
                description: &description,
                public,
                files: &pairs,
            })?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &gist, |w| out::one_gist(w, &gist))?;
        }
        GistCmd::Edit { id, file } => {
            let content = std::fs::read_to_string(&file)
                .map_err(|e| GiteeError::Usage(format!("read {file}: {e}")))?;
            let name = std::path::Path::new(&file)
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or_else(|| GiteeError::Usage(format!("invalid file path: {file}")))?
                .to_string();
            let pairs = [(name, content)];
            let gist = ctx
                .client
                .gists()
                .update(&id, &UpdateGist {
                    files: &pairs,
                    description: None,
                })?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &gist, |w| out::one_gist(w, &gist))?;
        }
        GistCmd::Delete { id, yes } => {
            confirm(&format!("Delete gist {id}"), yes)?;
            ctx.client.gists().delete(&id)?;
            let mut out = std::io::stdout().lock();
            writeln!(out, "Deleted gist {id}")?;
        }
    }
    Ok(())
}

/// Gitee requires a non-empty `description` (1–30 chars). When `--desc` is
/// omitted we default to the first file name, truncated to fit the limit.
fn truncate_description(desc: &str) -> String {
    let trimmed = desc.trim();
    if trimmed.is_empty() {
        return "gist".to_string();
    }
    if trimmed.chars().count() <= 30 {
        trimmed.to_string()
    } else {
        trimmed.chars().take(30).collect()
    }
}

fn read_gist_files(files: &[String], stdin_name: Option<&str>) -> Result<Vec<(String, String)>> {
    let mut out = Vec::with_capacity(files.len());
    for path in files {
        out.push(read_one_gist_file(path, stdin_name)?);
    }
    Ok(out)
}

fn read_one_gist_file(path: &str, stdin_name: Option<&str>) -> Result<(String, String)> {
    if path == "-" {
        let name = stdin_name.ok_or_else(|| {
            GiteeError::Usage("--filename is required when reading gist content from stdin".into())
        })?;
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(GiteeError::Io)?;
        return Ok((name.to_string(), buf));
    }
    let name = std::path::Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| GiteeError::Usage(format!("invalid file path: {path}")))?
        .to_string();
    let content = std::fs::read_to_string(path)
        .map_err(|e| GiteeError::Usage(format!("read {path}: {e}")))?;
    Ok((name, content))
}
