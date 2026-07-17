use crate::error::{GiteeError, Result};
use reqwest::blocking::Client as Http;
use serde::de::DeserializeOwned;
use serde_json::Value;

pub struct Client {
    http: Http,
    base: String,
    token: String,
}

impl Client {
    pub fn new(base: String, token: String) -> Self {
        let http = Http::builder()
            .gzip(true)
            .user_agent("gitee-cli/0.1")
            .build()
            .unwrap_or_default();
        Client { http, base, token }
    }

    fn full(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    fn check(&self, resp: reqwest::blocking::Response) -> Result<reqwest::blocking::Response> {
        let status = resp.status().as_u16();
        if resp.status().is_success() {
            Ok(resp)
        } else {
            let msg = resp.text().unwrap_or_default();
            Err(GiteeError::Api {
                status,
                message: msg,
            })
        }
    }

    pub fn get<T: DeserializeOwned>(&self, path: &str, query: &[(&str, &str)]) -> Result<T> {
        let mut q: Vec<(&str, &str)> = vec![("access_token", &self.token)];
        q.extend(query.iter().copied());
        let resp = self.http.get(self.full(path)).query(&q).send()?;
        self.check(resp)?.json().map_err(GiteeError::Http)
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
            let mut q: Vec<(&str, String)> = vec![
                ("access_token", self.token.clone()),
                ("page", page.to_string()),
                ("per_page", per.to_string()),
            ];
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
        let mut f: Vec<(&str, &str)> = vec![("access_token", &self.token)];
        f.extend(form.iter().copied());
        let req = match method {
            "POST" => self.http.post(self.full(path)),
            "PATCH" => self.http.patch(self.full(path)),
            _ => unreachable!(),
        };
        let resp = req.form(&f).send()?;
        self.check(resp)?.json().map_err(GiteeError::Http)
    }

    /// Issue create/update require a JSON body (Gitee rejects form on these);
    /// auth via `access_token` query param.
    pub fn patch_json<T: DeserializeOwned>(&self, path: &str, body: &Value) -> Result<T> {
        let resp = self
            .http
            .patch(self.full(path))
            .query(&[("access_token", self.token.as_str())])
            .json(body)
            .send()?;
        self.check(resp)?.json().map_err(GiteeError::Http)
    }

    /// For endpoints that return an empty body on success (e.g. PR review/merge).
    pub fn post_ok(&self, path: &str, form: &[(&str, &str)]) -> Result<()> {
        self.send_ok("POST", path, form)
    }

    pub fn put_ok(&self, path: &str, form: &[(&str, &str)]) -> Result<()> {
        self.send_ok("PUT", path, form)
    }

    fn send_ok(&self, method: &str, path: &str, form: &[(&str, &str)]) -> Result<()> {
        let mut f: Vec<(&str, &str)> = vec![("access_token", &self.token)];
        f.extend(form.iter().copied());
        let req = match method {
            "POST" => self.http.post(self.full(path)),
            "PUT" => self.http.put(self.full(path)),
            _ => unreachable!(),
        };
        let resp = req.form(&f).send()?;
        self.check(resp).map(|_| ())
    }
}
