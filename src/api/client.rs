use crate::error::{GiteeError, Result};
use super::search::Search;
use crate::repo::Repo;

use super::gists::Gists;
use reqwest::blocking::Client as Http;
use super::labels::Labels;
use serde::de::DeserializeOwned;
use super::milestones::Milestones;
use serde_json::Value;
use std::time::Duration;

use super::{issues::Issues, pulls::Pulls, releases::Releases, repos::Repos};

pub struct Client {
    http: Http,
    base: String,
    token: String,
    debug: bool,
}

/// Parameters for [`Client::raw`].
pub struct RawRequest<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub query: &'a [(&'a str, &'a str)],
    pub form: &'a [(&'a str, &'a str)],
    pub headers: &'a [(&'a str, &'a str)],
    pub body: Option<&'a [u8]>,
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

    pub fn search<'a>(&'a self) -> Search<'a> {
        Search::new(self)
    }

    pub fn releases<'a>(&'a self, repo: &'a Repo) -> Releases<'a> {
        Releases::new(self, repo)
    }

    pub fn gists<'a>(&'a self) -> Gists<'a> {
        Gists::new(self)
    }

    pub fn labels<'a>(&'a self, repo: &'a Repo) -> Labels<'a> {
        Labels::new(self, repo)
    }

    pub fn repos<'a>(&'a self) -> Repos<'a> {
        Repos::new(self)
    }

    pub fn milestones<'a>(&'a self, repo: &'a Repo) -> Milestones<'a> {
        Milestones::new(self, repo)
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
            .unwrap_or_else(|| Self::trim_cap(&body, 300));
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

    /// DELETE expecting an empty-body 2xx (204), e.g. gist/label/repo delete.
    pub fn delete_ok(&self, path: &str) -> Result<()> {
        self.send_ok("DELETE", path, &[])
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

    /// GET an absolute URL. Public assets are fetched without auth first; on
    /// 401 or 403 the request is retried with the Authorization header. reqwest
    /// follows redirects and forwards headers — note that a redirect to a host
    /// that rejects the forwarded token may still fail, so redirects are followed
    /// manually with auth only on `gitee.com` hosts (excluding the CDN).
    pub fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let mut url = url.to_string();
        let mut with_auth = false;
        for _ in 0..8 {
            if self.debug {
                eprintln!(
                    "-> GET {url}{}",
                    if with_auth { " (auth)" } else { "" }
                );
            }
            let mut req = self.http_no_redirect().get(&url);
            if with_auth {
                req = req.header("Authorization", self.auth());
            }
            let resp = req.send().map_err(GiteeError::Http)?;
            let status = resp.status();
            if status.is_success() {
                return resp.bytes().map(|b| b.to_vec()).map_err(GiteeError::Http);
            }
            let code = status.as_u16();
            if (code == 401 || code == 403) && !with_auth {
                with_auth = true;
                continue;
            }
            if status.is_redirection() {
                let loc = resp
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|v| v.to_str().ok())
                    .ok_or_else(|| GiteeError::Api {
                        status: code,
                        message: "redirect response missing Location header".into(),
                    })?;
                url = Self::resolve_location(&url, loc);
                with_auth = Self::asset_url_needs_auth(&url);
                continue;
            }
            return self.bytes_or_api_error(resp);
        }
        Err(GiteeError::Api {
            status: 0,
            message: "too many redirects fetching asset".into(),
        })
    }

    fn asset_url_needs_auth(url: &str) -> bool {
        url.contains("gitee.com") && !url.contains("foruda.gitee.com")
    }

    fn resolve_location(base: &str, loc: &str) -> String {
        if loc.starts_with("http://") || loc.starts_with("https://") {
            return loc.to_string();
        }
        let base_url = reqwest::Url::parse(base).expect("asset base url");
        base_url.join(loc).expect("redirect location").to_string()
    }

    fn http_no_redirect(&self) -> Http {
        // Gitee's release asset endpoints reject the API user-agent on auth'd downloads.
        Http::builder()
            .gzip(true)
            .timeout(Duration::from_secs(30))
            .user_agent("curl/8.5.0")
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("reqwest client")
    }


    fn bytes_or_api_error(&self, resp: reqwest::blocking::Response) -> Result<Vec<u8>> {
        let status = resp.status();
        if status.is_success() {
            return resp.bytes().map(|b| b.to_vec()).map_err(GiteeError::Http);
        }
        let code = status.as_u16();
        let body = resp.text().unwrap_or_default();
        let message = Self::trim_cap(&body, 2048);
        Err(GiteeError::Api {
            status: code,
            message,
        })
    }


    fn send_ok(&self, method: &str, path: &str, form: &[(&str, &str)]) -> Result<()> {
        self.trace(method, path);
        let req = match method {
            "POST" => self.http.post(self.full(path)),
            "PUT" => self.http.put(self.full(path)),
            "DELETE" => self.http.delete(self.full(path)),
            _ => unreachable!(),
        };
        let resp = req.header("Authorization", self.auth()).form(form).send()?;
        self.check(resp, method, path).map(|_| ())
    }

    /// Char-safe truncation for error bodies (CJK messages would panic a
    /// byte-slice cap like `&t[..max]`).
    fn trim_cap(s: &str, max: usize) -> String {
        let t = s.trim();
        if t.chars().count() <= max {
            return t.to_string();
        }
        format!("{}…", t.chars().take(max).collect::<String>())
    }

    /// Issue a raw API request and return the response body text.
    pub fn raw(&self, req: &RawRequest<'_>) -> Result<String> {
        self.trace(req.method, req.path);
        let method = req.method.to_uppercase();
        let url = self.full(req.path);

        let mut rb = match method.as_str() {
            "GET" => self.http.get(&url),
            "POST" => self.http.post(&url),
            "PUT" => self.http.put(&url),
            "PATCH" => self.http.patch(&url),
            "DELETE" => self.http.delete(&url),
            "HEAD" => self.http.head(&url),
            _ => unreachable!("method validated before raw()"),
        };

        rb = rb.header("Authorization", self.auth());

        if matches!(method.as_str(), "GET" | "HEAD" | "DELETE") {
            let mut q: Vec<(&str, &str)> = req.query.to_vec();
            q.extend_from_slice(req.form);
            if !q.is_empty() {
                rb = rb.query(&q);
            }
        } else if let Some(body) = req.body {
            let has_ct = req
                .headers
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("content-type"));
            if !has_ct {
                rb = rb.header("Content-Type", "application/json");
            }
            rb = rb.body(body.to_vec());
            if !req.query.is_empty() {
                rb = rb.query(req.query);
            }
        } else if !req.form.is_empty() {
            rb = rb.form(req.form);
            if !req.query.is_empty() {
                rb = rb.query(req.query);
            }
        } else if !req.query.is_empty() {
            rb = rb.query(req.query);
        }

        for (k, v) in req.headers {
            rb = rb.header(*k, *v);
        }

        let resp = rb.send()?;
        let status = resp.status();
        if status.is_success() {
            return Ok(resp.text().unwrap_or_default());
        }

        let code = status.as_u16();
        let body = resp.text().unwrap_or_default();
        let message = Self::trim_cap(&body, 2048);
        if self.debug {
            eprintln!("<- {} {} -> {code}: {message}", req.method, req.path);
        }
        Err(GiteeError::Api {
            status: code,
            message,
        })
    }

    /// GET-only pagination: walk `page`/`per_page=100` until a short page.
    pub fn raw_paged(
        &self,
        path: &str,
        query: &[(&str, &str)],
        headers: &[(&str, &str)],
    ) -> Result<Vec<Value>> {
        let mut out: Vec<Value> = Vec::new();
        let mut page = 1u32;
        let per = 100;
        loop {
            let mut q: Vec<(String, String)> = vec![
                ("page".into(), page.to_string()),
                ("per_page".into(), per.to_string()),
            ];
            for (k, v) in query {
                q.push((k.to_string(), v.to_string()));
            }
            let qref: Vec<(&str, &str)> = q.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
            let body = self.raw(&RawRequest {
                method: "GET",
                path,
                query: &qref,
                form: &[],
                headers,
                body: None,
            })?;
            let parsed: Value = serde_json::from_str(&body).map_err(|e| {
                GiteeError::Usage(format!("--paginate requires JSON array responses: {e}"))
            })?;
            let arr = parsed.as_array().ok_or_else(|| {
                GiteeError::Usage("--paginate requires JSON array responses".into())
            })?;
            let n = arr.len();
            out.extend(arr.iter().cloned());
            if n < per {
                break;
            }
            page += 1;
        }
        Ok(out)
    }
}
