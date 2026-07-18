use std::fs;
use std::io::{self, Read};
use std::path::Path;

use serde_json::Value;

use crate::api::client::{Client, RawRequest};
use crate::cli::ApiArgs;
use crate::error::{GiteeError, Result};

const METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD"];

pub fn execute(client: &Client, args: ApiArgs) -> Result<()> {
    let path = normalize_endpoint(&args.endpoint)?;

    let mut form: Vec<(String, String)> = Vec::new();
    for f in &args.fields {
        let (k, v) = parse_kv(f)?;
        form.push((k.to_string(), v.to_string()));
    }
    for f in &args.raw_fields {
        let (k, v) = parse_kv(f)?;
        form.push((k.to_string(), v.to_string()));
    }

    let mut header_pairs: Vec<(String, String)> = Vec::new();
    for h in &args.headers {
        let (k, v) = parse_header(h)?;
        header_pairs.push((k.to_string(), v.to_string()));
    }

    let body_bytes = if let Some(input) = &args.input {
        Some(read_input(input)?)
    } else {
        None
    };

    let has_payload = !form.is_empty() || body_bytes.is_some();
    let method = effective_method(args.method.as_deref(), has_payload)?;

    if args.paginate && method != "GET" {
        return Err(GiteeError::Usage("--paginate requires GET".into()));
    }

    let form_refs: Vec<(&str, &str)> = form.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let header_refs: Vec<(&str, &str)> = header_pairs
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    if args.paginate {
        // GET: -F fields belong on the query string (same as a plain raw GET).
        let values = client.raw_paged(&path, &form_refs, &header_refs)?;
        let text = serde_json::to_string_pretty(&values)
            .map_err(|e| GiteeError::Usage(e.to_string()))?;
        println!("{text}");
        return Ok(());
    }

    let body_ref = body_bytes.as_deref();
    let response = client.raw(&RawRequest {
        method: &method,
        path: &path,
        query: &[],
        form: &form_refs,
        headers: &header_refs,
        body: body_ref,
    })?;
    emit_stdout(&response)?;
    Ok(())
}

fn emit_stdout(body: &str) -> Result<()> {
    let text = match serde_json::from_str::<Value>(body) {
        Ok(v) => serde_json::to_string_pretty(&v).map_err(|e| GiteeError::Usage(e.to_string()))?,
        Err(_) => body.to_string(),
    };
    if text.ends_with('\n') {
        print!("{text}");
    } else {
        println!("{text}");
    }
    Ok(())
}

fn read_input(spec: &str) -> Result<Vec<u8>> {
    if spec == "-" {
        let mut buf = Vec::new();
        io::stdin()
            .read_to_end(&mut buf)
            .map_err(|e| GiteeError::Usage(format!("read stdin: {e}")))?;
        return Ok(buf);
    }
    fs::read(Path::new(spec)).map_err(|e| GiteeError::Usage(format!("read file {spec}: {e}")))
}

pub(crate) fn normalize_endpoint(endpoint: &str) -> Result<String> {
    let trimmed = endpoint.trim();
    if trimmed.is_empty() {
        return Err(GiteeError::Usage("endpoint required".into()));
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return Err(GiteeError::Usage(
            "pass an API path, not a full URL (e.g. `user` or `/repos/owner/repo`)".into(),
        ));
    }
    let mut path = trimmed.trim_start_matches('/').to_string();
    if path.starts_with("api/v5/") {
        path = path["api/v5/".len()..].to_string();
    } else if path == "api/v5" {
        path.clear();
    }
    if path.is_empty() {
        Ok("/".to_string())
    } else {
        Ok(format!("/{path}"))
    }
}

pub(crate) fn effective_method(explicit: Option<&str>, has_payload: bool) -> Result<String> {
    match explicit {
        Some(m) => {
            let upper = m.to_uppercase();
            if METHODS.contains(&upper.as_str()) {
                Ok(upper)
            } else {
                Err(GiteeError::Usage(format!(
                    "unsupported HTTP method '{m}'; use one of: {}",
                    METHODS.join(", ")
                )))
            }
        }
        None if has_payload => Ok("POST".into()),
        None => Ok("GET".into()),
    }
}

pub(crate) fn parse_kv(s: &str) -> Result<(&str, &str)> {
    let (k, v) = s
        .split_once('=')
        .ok_or_else(|| GiteeError::Usage(format!("expected key=value, got '{s}'")))?;
    Ok((k, v))
}

pub(crate) fn parse_header(s: &str) -> Result<(&str, &str)> {
    let (k, v) = s
        .split_once(':')
        .ok_or_else(|| GiteeError::Usage(format!("expected 'Name: value', got '{s}'")))?;
    let key = k.trim();
    if key.is_empty() {
        return Err(GiteeError::Usage(format!(
            "expected 'Name: value', got '{s}'"
        )));
    }
    Ok((key, v.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_endpoint_paths() {
        assert_eq!(normalize_endpoint("user").unwrap(), "/user");
        assert_eq!(normalize_endpoint("/user").unwrap(), "/user");
        assert_eq!(normalize_endpoint("/api/v5/user").unwrap(), "/user");
        assert_eq!(normalize_endpoint("api/v5/user").unwrap(), "/user");
        assert_eq!(normalize_endpoint("  /api/v5/repos/o/r  ").unwrap(), "/repos/o/r");
    }

    #[test]
    fn normalize_endpoint_rejects_urls() {
        assert!(normalize_endpoint("https://gitee.com/api/v5/user").is_err());
        assert!(normalize_endpoint("http://example.com/user").is_err());
    }

    #[test]
    fn effective_method_defaults() {
        assert_eq!(effective_method(None, false).unwrap(), "GET");
        assert_eq!(effective_method(None, true).unwrap(), "POST");
        assert_eq!(effective_method(Some("patch"), false).unwrap(), "PATCH");
    }

    #[test]
    fn effective_method_rejects_unknown() {
        assert!(effective_method(Some("TRACE"), false).is_err());
    }

    #[test]
    fn parse_kv_splits_on_first_equals() {
        assert_eq!(parse_kv("a=b=c").unwrap(), ("a", "b=c"));
        assert!(parse_kv("nope").is_err());
    }

    #[test]
    fn parse_header_splits_and_trims() {
        assert_eq!(parse_header("X-Foo: bar").unwrap(), ("X-Foo", "bar"));
        assert_eq!(parse_header("X-Foo:  bar baz ").unwrap(), ("X-Foo", "bar baz"));
        assert!(parse_header(": missing").is_err());
        assert!(parse_header("no-colon").is_err());
    }
}
