use std::io::Write;
use std::process::Command;

use inquire::Text;

use crate::config::Config;
use crate::error::{GiteeError, Result};

/// True when the user omitted `--title` and did not use `--fill` (PR only).
pub fn should_run_interactive_create(title: Option<&str>, fill: bool) -> bool {
    title.is_none() && !fill
}

pub fn missing_title_usage(subcommand: &str, mention_fill: bool) -> GiteeError {
    if mention_fill {
        GiteeError::Usage(format!("{subcommand} needs --title (or --fill)"))
    } else {
        GiteeError::Usage(format!("{subcommand} needs --title"))
    }
}

/// Resolve the editor executable: `$VISUAL`, then `$EDITOR`, then config `editor`.
pub fn resolve_editor_command(
    visual: Option<&str>,
    editor_env: Option<&str>,
    config_editor: Option<&str>,
) -> Option<String> {
    visual
        .filter(|s| !s.trim().is_empty())
        .or_else(|| editor_env.filter(|s| !s.trim().is_empty()))
        .or_else(|| config_editor.filter(|s| !s.trim().is_empty()))
        .map(str::to_string)
}

pub fn prompt_title(default: Option<&str>) -> Result<String> {
    let mut prompt = Text::new("Title");
    if let Some(d) = default.filter(|s| !s.is_empty()) {
        prompt = prompt.with_default(d);
    }
    let title = prompt
        .prompt()
        .map_err(|e| GiteeError::Usage(format!("prompt cancelled: {e}")))?;
    let title = title.trim().to_string();
    if title.is_empty() {
        return Err(GiteeError::Usage("title is required".into()));
    }
    Ok(title)
}

/// Open `initial` in an external editor; returns `None` when the result is empty.
pub fn edit_body_in_editor(initial: &str, editor_cmd: &str) -> Result<Option<String>> {
    use shell_words::split;

    let mut file = tempfile::NamedTempFile::new()?;
    file.write_all(initial.as_bytes())?;
    file.flush()?;
    let path = file.path().to_owned();

    let mut parts: Vec<String> = split(editor_cmd)
        .map_err(|e| GiteeError::Usage(format!("invalid editor command: {e}")))?;
    if parts.is_empty() {
        return Err(GiteeError::Usage("editor command is empty".into()));
    }
    parts.push(path.to_string_lossy().into_owned());

    let program = &parts[0];
    let args = &parts[1..];
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|e| GiteeError::Usage(format!("failed to run editor `{program}`: {e}")))?;
    if !status.success() {
        return Err(GiteeError::Usage("editor exited with an error".into()));
    }

    let body = std::fs::read_to_string(&path)?;
    let trimmed = body.trim();
    Ok((!trimmed.is_empty()).then(|| trimmed.to_string()))
}

pub fn resolve_editor_from_env_and_config() -> Result<String> {
    let settings = Config::load_settings()?;
    resolve_editor_command(
        std::env::var("VISUAL").ok().as_deref(),
        std::env::var("EDITOR").ok().as_deref(),
        settings.editor.as_deref(),
    )
    .ok_or_else(|| {
        GiteeError::Usage(
            "no editor: set $VISUAL, $EDITOR, or `gitee config set editor <cmd>`".into(),
        )
    })
}

pub fn stdin_is_tty() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_run_interactive_when_title_missing_and_not_fill() {
        assert!(should_run_interactive_create(None, false));
        assert!(!should_run_interactive_create(Some("t"), false));
        assert!(!should_run_interactive_create(None, true));
        assert!(!should_run_interactive_create(Some("t"), true));
    }

    #[test]
    fn missing_title_usage_messages() {
        assert_eq!(
            missing_title_usage("issue create", false).to_string(),
            "issue create needs --title"
        );
        assert_eq!(
            missing_title_usage("pr create", true).to_string(),
            "pr create needs --title (or --fill)"
        );
    }

    #[test]
    fn resolve_editor_prefers_visual_then_editor_then_config() {
        assert_eq!(
            resolve_editor_command(Some("visual"), Some("editor"), Some("config")),
            Some("visual".into())
        );
        assert_eq!(
            resolve_editor_command(None, Some("editor"), Some("config")),
            Some("editor".into())
        );
        assert_eq!(
            resolve_editor_command(None, None, Some("config")),
            Some("config".into())
        );
        assert_eq!(resolve_editor_command(None, None, None), None);
        assert_eq!(
            resolve_editor_command(Some(""), Some("editor"), None),
            Some("editor".into())
        );
        assert_eq!(
            resolve_editor_command(Some("  "), None, Some("config")),
            Some("config".into())
        );
    }
}
