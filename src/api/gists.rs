use super::client::Client;
use crate::error::Result;
use crate::models::Gist;

pub struct Gists<'a> {
    client: &'a Client,
}

pub struct CreateGist<'a> {
    pub description: &'a str,
    pub public: bool,
    pub files: &'a [(String, String)],
}

pub struct UpdateGist<'a> {
    pub files: &'a [(String, String)],
    pub description: Option<&'a str>,
}

impl Gists<'_> {
    pub(crate) fn new(client: &Client) -> Gists<'_> {
        Gists { client }
    }

    pub fn list(&self, limit: usize) -> Result<Vec<Gist>> {
        self.client.get_paged("/gists", &[], limit)
    }

    pub fn get(&self, id: &str) -> Result<Gist> {
        self.client.get(&format!("/gists/{id}"), &[])
    }

    /// Gitee gist create uses Rails-style urlencoded nested fields:
    /// `files[<name>][content]=<text>`, plus required `description` (1–30 chars)
    /// and `public` sent as the string `"true"` or `"false"`.
    pub fn create(&self, req: &CreateGist<'_>) -> Result<Gist> {
        let mut pairs = vec![
            ("description".to_string(), req.description.to_string()),
            (
                "public".to_string(),
                if req.public {
                    "true".to_string()
                } else {
                    "false".to_string()
                },
            ),
        ];
        push_file_fields(&mut pairs, req.files);
        let form = form_refs(&pairs);
        self.client.post("/gists", &form)
    }

    pub fn update(&self, id: &str, req: &UpdateGist<'_>) -> Result<Gist> {
        let mut pairs: Vec<(String, String)> = Vec::new();
        if let Some(d) = req.description {
            pairs.push(("description".to_string(), d.to_string()));
        }
        push_file_fields(&mut pairs, req.files);
        let form = form_refs(&pairs);
        self.client.patch(&format!("/gists/{id}"), &form)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.client.delete_ok(&format!("/gists/{id}"))
    }
}

fn push_file_fields(pairs: &mut Vec<(String, String)>, files: &[(String, String)]) {
    for (name, content) in files {
        pairs.push((format!("files[{name}][content]"), content.clone()));
    }
}

fn form_refs(pairs: &[(String, String)]) -> Vec<(&str, &str)> {
    pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect()
}
