use serde::Deserialize;

use crate::error::GoogleError;

const DRIVE_BASE: &str = "https://www.googleapis.com/drive/v3";
const GOOGLE_DOC_MIME: &str = "application/vnd.google-apps.document";

const FILE_FIELDS: &str = "id,name,mimeType,modifiedTime,webViewLink";

/// A normalized Drive file. `removed` marks files that were trashed/deleted
/// (surfaced by the changes feed) so the caller can skip or tombstone them.
pub struct DriveFile {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub modified_time: Option<String>,
    pub web_view_link: Option<String>,
    pub removed: bool,
    pub raw: serde_json::Value,
}

/// Result of a changes-feed poll: the changed files plus the token to persist
/// for the next incremental sync.
pub struct DriveChanges {
    pub files: Vec<DriveFile>,
    pub new_start_page_token: Option<String>,
}

/// Read-only Drive API client bound to a single access token.
pub struct DriveClient {
    http: reqwest::Client,
    access_token: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawFile {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    mime_type: Option<String>,
    #[serde(default)]
    modified_time: Option<String>,
    #[serde(default)]
    web_view_link: Option<String>,
}

impl RawFile {
    fn into_drive_file(self, removed: bool) -> DriveFile {
        let raw = serde_json::json!({
            "id": self.id,
            "name": self.name,
            "mimeType": self.mime_type,
            "modifiedTime": self.modified_time,
            "webViewLink": self.web_view_link,
        });
        DriveFile {
            id: self.id,
            name: self.name.unwrap_or_default(),
            mime_type: self.mime_type.unwrap_or_default(),
            modified_time: self.modified_time,
            web_view_link: self.web_view_link,
            removed,
            raw,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FilesListResponse {
    #[serde(default)]
    files: Vec<RawFile>,
    #[serde(default)]
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartTokenResponse {
    start_page_token: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Change {
    #[serde(default)]
    removed: bool,
    #[serde(default)]
    file: Option<RawFile>,
    #[serde(default)]
    file_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChangesResponse {
    #[serde(default)]
    changes: Vec<Change>,
    #[serde(default)]
    next_page_token: Option<String>,
    #[serde(default)]
    new_start_page_token: Option<String>,
}

impl DriveClient {
    pub fn new(access_token: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            access_token: access_token.into(),
        }
    }

    /// Lists files matching `query` (Drive query syntax), following pagination
    /// up to `max` files. Used for the initial seed sync.
    pub async fn list_files(
        &self,
        query: Option<&str>,
        max: usize,
    ) -> Result<Vec<DriveFile>, GoogleError> {
        let fields = format!("files({FILE_FIELDS}),nextPageToken");
        let mut out = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut req = self
                .http
                .get(format!("{DRIVE_BASE}/files"))
                .bearer_auth(&self.access_token)
                .query(&[("fields", fields.as_str()), ("pageSize", "100")]);
            if let Some(q) = query {
                req = req.query(&[("q", q)]);
            }
            if let Some(ref t) = page_token {
                req = req.query(&[("pageToken", t.as_str())]);
            }

            let res = req.send().await?;
            let body: FilesListResponse = parse_json(res).await?;
            out.extend(body.files.into_iter().map(|f| f.into_drive_file(false)));

            if out.len() >= max {
                out.truncate(max);
                break;
            }
            match body.next_page_token {
                Some(t) => page_token = Some(t),
                None => break,
            }
        }

        Ok(out)
    }

    /// Fetches the current start page token, marking the point from which future
    /// `list_changes` calls report deltas.
    pub async fn start_page_token(&self) -> Result<String, GoogleError> {
        let res = self
            .http
            .get(format!("{DRIVE_BASE}/changes/startPageToken"))
            .bearer_auth(&self.access_token)
            .send()
            .await?;
        let body: StartTokenResponse = parse_json(res).await?;
        Ok(body.start_page_token)
    }

    /// Polls the changes feed starting at `page_token`, following pagination and
    /// returning the changed files plus the token to store for next time.
    pub async fn list_changes(&self, page_token: &str) -> Result<DriveChanges, GoogleError> {
        let fields = format!(
            "changes(removed,fileId,file({FILE_FIELDS})),nextPageToken,newStartPageToken"
        );
        let mut files = Vec::new();
        let mut token = page_token.to_string();

        let new_start = loop {
            let res = self
                .http
                .get(format!("{DRIVE_BASE}/changes"))
                .bearer_auth(&self.access_token)
                .query(&[
                    ("pageToken", token.as_str()),
                    ("fields", fields.as_str()),
                    ("pageSize", "100"),
                ])
                .send()
                .await?;
            let body: ChangesResponse = parse_json(res).await?;

            for change in body.changes {
                match change.file {
                    Some(f) => files.push(f.into_drive_file(change.removed)),
                    None if change.removed => {
                        if let Some(id) = change.file_id {
                            files.push(DriveFile {
                                id: id.clone(),
                                name: String::new(),
                                mime_type: String::new(),
                                modified_time: None,
                                web_view_link: None,
                                removed: true,
                                raw: serde_json::json!({ "id": id, "removed": true }),
                            });
                        }
                    }
                    None => {}
                }
            }

            if let Some(t) = body.next_page_token {
                token = t;
                continue;
            }
            break body.new_start_page_token;
        };

        Ok(DriveChanges {
            files,
            new_start_page_token: new_start,
        })
    }

    /// Exports a Google Doc as plain text. Non-Google-Docs return an empty
    /// string (binary/native files aren't downloaded in this phase).
    pub async fn export_text(&self, file: &DriveFile) -> Result<String, GoogleError> {
        if file.mime_type != GOOGLE_DOC_MIME {
            return Ok(String::new());
        }

        let res = self
            .http
            .get(format!("{DRIVE_BASE}/files/{}/export", file.id))
            .bearer_auth(&self.access_token)
            .query(&[("mimeType", "text/plain")])
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status().as_u16();
            return Err(GoogleError::Api(status, res.text().await.unwrap_or_default()));
        }
        Ok(res.text().await?)
    }
}

async fn parse_json<T: serde::de::DeserializeOwned>(
    res: reqwest::Response,
) -> Result<T, GoogleError> {
    if !res.status().is_success() {
        let status = res.status().as_u16();
        return Err(GoogleError::Api(status, res.text().await.unwrap_or_default()));
    }
    Ok(res.json::<T>().await?)
}
