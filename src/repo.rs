use crate::error::{GiteeError, Result};

#[derive(Clone, Debug)]
pub struct Repo {
    pub owner: String,
    pub name: String,
}

impl Repo {
    pub fn resolve(explicit: Option<&str>, remote: Option<&str>) -> Result<Repo> {
        if let Some(s) = explicit {
            return Self::from_spec(s);
        }
        let remote = remote.unwrap_or("origin");
        let url = std::process::Command::new("git")
            .args(["remote", "get-url", remote])
            .output()
            .map_err(|e| GiteeError::RepoResolve(format!("git: {e}")))?;
        if !url.status.success() {
            return Err(GiteeError::RepoResolve(format!("no '{remote}' remote found")));
        }
        let raw = String::from_utf8_lossy(&url.stdout);
        Self::parse_url(raw.trim())
    }

    /// Parse either an `owner/name` pair or a git URL (SSH or HTTPS).
    pub fn from_spec(s: &str) -> Result<Repo> {
        let s = s.trim();
        if s.contains("://") || s.starts_with("git@") {
            Self::parse_url(s)
        } else {
            Self::parse_pair(s)
        }
    }

    fn parse_pair(s: &str) -> Result<Repo> {
        let (owner, name) = s
            .split_once('/')
            .ok_or_else(|| GiteeError::RepoResolve(format!("expected owner/repo, got '{s}'")))?;
        Ok(Repo {
            owner: owner.trim().to_string(),
            name: name.trim().trim_end_matches(".git").to_string(),
        })
    }

    fn parse_url(u: &str) -> Result<Repo> {
        let path = match url::Url::parse(u) {
            Ok(parsed) => parsed.path().to_string(),
            Err(_) => u.split_once(':').map(|x| x.1).unwrap_or(u).to_string(),
        };
        let path = path.trim_start_matches('/').trim_end_matches(".git");
        Self::parse_pair(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(u: &str) -> Repo {
        Repo::parse_url(u).expect("should parse")
    }

    #[test]
    fn parses_ssh_url() {
        let r = parse("git@gitee.com:oschina/git.git");
        assert_eq!(r.owner, "oschina");
        assert_eq!(r.name, "git");
    }

    #[test]
    fn parses_https_url() {
        let r = parse("https://gitee.com/oschina/git.git");
        assert_eq!(r.owner, "oschina");
        assert_eq!(r.name, "git");
    }

    #[test]
    fn parses_https_with_credentials() {
        let r = parse("https://oauth2:TOKEN@gitee.com/oschina/git");
        assert_eq!(r.owner, "oschina");
        assert_eq!(r.name, "git");
    }

    #[test]
    fn parses_pair() {
        let r = Repo::parse_pair("oschina/git").expect("should parse");
        assert_eq!(r.owner, "oschina");
        assert_eq!(r.name, "git");
    }

    #[test]
    fn from_spec_dispatches() {
        let a = Repo::from_spec("oschina/git").unwrap();
        assert_eq!(a.owner, "oschina");
        let b = Repo::from_spec("git@gitee.com:oschina/git.git").unwrap();
        assert_eq!(b.owner, "oschina");
    }
}
