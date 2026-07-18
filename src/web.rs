use crate::error::Result;
use crate::repo::Repo;
use std::io::Write;

pub fn repo_url(host: &str, repo: &Repo) -> String {
    format!("https://{host}/{}/{}", repo.owner, repo.name)
}

pub fn pull_url(host: &str, repo: &Repo, number: i64) -> String {
    format!("{}/pulls/{number}", repo_url(host, repo))
}

pub fn issue_url(host: &str, repo: &Repo, ident: &str) -> String {
    format!("{}/issues/{ident}", repo_url(host, repo))
}

pub fn release_url(host: &str, repo: &Repo, tag: &str) -> String {
    format!("{}/releases/tag/{tag}", repo_url(host, repo))
}

/// Open `url` in a browser when possible; on failure (headless / no opener)
/// print the URL and succeed.
pub fn open_or_print(url: &str) -> Result<()> {
    match open::that(url) {
        Ok(()) => Ok(()),
        Err(_) => {
            let mut out = std::io::stdout().lock();
            writeln!(out, "{url}")?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::Repo;

    fn repo() -> Repo {
        Repo {
            owner: "oschina".into(),
            name: "gitee-cli".into(),
        }
    }

    #[test]
    fn builds_ticket_url_shapes() {
        let r = repo();
        assert_eq!(
            repo_url("gitee.com", &r),
            "https://gitee.com/oschina/gitee-cli"
        );
        assert_eq!(
            pull_url("gitee.com", &r, 12),
            "https://gitee.com/oschina/gitee-cli/pulls/12"
        );
        assert_eq!(
            issue_url("gitee.com", &r, "I6D3AV"),
            "https://gitee.com/oschina/gitee-cli/issues/I6D3AV"
        );
        assert_eq!(
            release_url("gitee.com", &r, "v1.2.3"),
            "https://gitee.com/oschina/gitee-cli/releases/tag/v1.2.3"
        );
    }
}
