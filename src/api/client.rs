use crate::error::{GiteeError, Result};
use crate::repo::Repo;
use reqwest::blocking::Client as Http;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::time::Duration;

use super::{issues::Issues, pulls::Pulls, releases::Releases, repos::Repos};

pub struct Client {
    http: Http,
    base: String,
    token: String,
    debug: bool,
}

impl Client {
    pub fn new(base: String, token: String) -> Self {
        let http = Http::builder()
            .gzip(true)
            .timeout(Duration::from_secs(30))
            .user_agent("gitee-cli/0.1")
            .build()
            .expect("reqwest client");
        Client {
            http,
            base,
            token,
            debug: false,
        }
    }

    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }

    pub fn for_host(host: &str, token: String) -> Self {
        Self::new(format!("https://{host}/api/v5"), token)
    }

    pub fn pulls<'a>(&'a self, repo: &'a Repo) -> Pulls<'a> {
        Pulls::new(self, repo)
    }

    pub fn issues<'a>(&'a self, repo: &'a Repo) -> Issues<'a> {
        Issues::new(self, repo)
    }

    pub fn releases<'a>(&'a self, repo: &'a Repo) -> Releases<'a> {
        Releases::new(self, repo)
    }

    pub fn repos<'a>(&'a self) -> Repos<'a> {
        Repos::new(self)
    }

    pub(crate) fn str_refs<'a>(pairs: &'a [(&'a str, String)]) -> Vec<(&'a str, &'a str)> {
        pairs.iter().map(|(k, v)| (*k, v.as_str())).collect()
    }

    /// Gitee accepts `Authorization: token <T>`. Sending the token in the header
    /// keeps it out of URLs/query strings, and therefore out of reqwest error
    /// messages and server/proxy access logs.
    fn auth(&self) -> String {
        format!("token {}", self.token)
    }

    fn full(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    /// Map a non-2xx response onto a typed error. Gitee error bodies are JSON
    /// envelopes like `{"message":"..."}`; we extract the human message so users
    /// see something actionable instead of a raw JSON blob. 401 and 404 get
    /// dedicated variants for clearer guidance.
    fn check(
        &self,
        resp: reqwest::blocking::Response,
        method: &str,
        path: &str,
    ) -> Result<reqwest::blocking::Response> {
        let status = resp.status();
        if status.is_success() {
            return Ok(resp);
        }
        let code = status.as_u16();
        let body = resp.text().unwrap_or_default();
        let message = serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|v| {
                v.get("message")
                    .and_then(|m| m.as_str())
                    .map(str::to_owned)
                    .or_else(|| v.get("error").and_then(|m| m.as_str()).map(str::to_owned))
            })
            .unwrap_or_else(|| {
                let t = body.trim();
                if t.len() > 300 {
                    format!("{}…", &t[..300])
                } else {
                    t.to_string()
                }
            });
        if self.debug {
            eprintln!("<- {method} {path} -> {code}: {message}");
        }
        Err(match code {
            401 => GiteeError::Unauthorized,
            404 => GiteeError::NotFound(path.to_string()),
            _ => GiteeError::Api {
                status: code,
                message,
            },
        })
    }

    fn trace(&self, method: &str, path: &str) {
        if self.debug {
            eprintln!("-> {method} {path}");
        }
    }

    pub fn get<T: DeserializeOwned>(&self, path: &str, query: &[(&str, &str)]) -> Result<T> {
        self.trace("GET", path);
        let resp = self
            .http
            .get(self.full(path))
            .header("Authorization", self.auth())
            .query(query)
            .send()?;
        self.check(resp, "GET", path)?
            .json()
            .map_err(GiteeError::Http)
    }

    pub fn get_paged<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
        limit: usize,
    ) -> Result<Vec<T>> {
        let mut out: Vec<T> = Vec::new();
        let mut page = 1u32;
        let per = 100;
        while out.len() < limit {
            let mut q: Vec<(&str, String)> =
                vec![("page", page.to_string()), ("per_page", per.to_string())];
            for (k, v) in query {
                q.push((k, v.to_string()));
            }
            let qref: Vec<(&str, &str)> = q.iter().map(|(k, v)| (*k, v.as_str())).collect();
            let chunk: Vec<T> = self.get(path, &qref)?;
            let n = chunk.len();
            out.extend(chunk);
            if n < per {
                break;
            }
            page += 1;
        }
        if out.len() > limit {
            out.truncate(limit);
        }
        Ok(out)
    }

    pub fn post<T: DeserializeOwned>(&self, path: &str, form: &[(&str, &str)]) -> Result<T> {
        self.send("POST", path, form)
    }

    pub fn patch<T: DeserializeOwned>(&self, path: &str, form: &[(&str, &str)]) -> Result<T> {
        self.send("PATCH", path, form)
    }

    fn send<T: DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        form: &[(&str, &str)],
    ) -> Result<T> {
        self.trace(method, path);
        let req = match method {
            "POST" => self.http.post(self.full(path)),
            "PATCH" => self.http.patch(self.full(path)),
            _ => unreachable!(),
        };
        let resp = req.header("Authorization", self.auth()).form(form).send()?;
        self.check(resp, method, path)?
            .json()
            .map_err(GiteeError::Http)
    }

    /// Issue update requires a JSON body (Gitee rejects form encoding here).
    pub fn patch_json<T: DeserializeOwned>(&self, path: &str, body: &Value) -> Result<T> {
        self.trace("PATCH", path);
        let resp = self
            .http
            .patch(self.full(path))
            .header("Authorization", self.auth())
            .json(body)
            .send()?;
        self.check(resp, "PATCH", path)?
            .json()
            .map_err(GiteeError::Http)
    }

    /// For endpoints that return an empty body on success (e.g. PR review/merge).
    pub fn post_ok(&self, path: &str, form: &[(&str, &str)]) -> Result<()> {
        self.send_ok("POST", path, form)
    }

    pub fn put_ok(&self, path: &str, form: &[(&str, &str)]) -> Result<()> {
        self.send_ok("PUT", path, form)
    }

    pub fn post_multipart<T: DeserializeOwned>(&self, path: &str, file_path: &str) -> Result<T> {
        self.trace("POST", path);
        let form = reqwest::blocking::multipart::Form::new()
            .file("file", file_path)
            .map_err(|e| GiteeError::Usage(format!("read file {file_path}: {e}")))?;
        let resp = self
            .http
            .post(self.full(path))
            .header("Authorization", self.auth())
            .multipart(form)
            .send()?;
        self.check(resp, "POST", path)?
            .json()
            .map_err(GiteeError::Http)
    }

    fn send_ok(&self, method: &str, path: &str, form: &[(&str, &str)]) -> Result<()> {
        self.trace(method, path);
        let req = match method {
            "POST" => self.http.post(self.full(path)),
            "PUT" => self.http.put(self.full(path)),
            _ => unreachable!(),
        };
        let resp = req.header("Authorization", self.auth()).form(form).send()?;
        self.check(resp, method, path).map(|_| ())
    }
}
