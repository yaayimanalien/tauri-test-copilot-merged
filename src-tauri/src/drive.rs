use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    time::timeout,
};

const SCOPE: &str = "https://www.googleapis.com/auth/drive.appdata";
const FILES_URL: &str = "https://www.googleapis.com/drive/v3/files";
const CLIENT_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMetadata {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub app_properties: HashMap<String, String>,
}

#[derive(Deserialize)]
struct FileList {
    #[serde(default)]
    files: Vec<FileMetadata>,
    #[serde(default)]
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
}

pub struct DriveClient {
    http: reqwest::Client,
    access_token: String,
}

fn http_error(action: &str, status: reqwest::StatusCode) -> String {
    format!("Google Drive {action} failed ({status})")
}

pub async fn oauth_connect(client_id: &str, client_secret: &str) -> Result<String, String> {
    let client_id = client_id.to_owned();
    let client_secret = client_secret.to_owned();
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .map_err(|e| format!("Couldn't start Google sign-in: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("Couldn't read Google sign-in address: {e}"))?
        .port();
    let redirect_uri = format!("http://127.0.0.1:{port}");
    let mut verifier_bytes = [0u8; 48];
    let mut state_bytes = [0u8; 16];
    OsRng.fill_bytes(&mut verifier_bytes);
    OsRng.fill_bytes(&mut state_bytes);
    let verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);
    let state = URL_SAFE_NO_PAD.encode(state_bytes);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));

    let mut authorization = reqwest::Url::parse("https://accounts.google.com/o/oauth2/v2/auth")
        .map_err(|e| format!("Couldn't create Google sign-in URL: {e}"))?;
    authorization
        .query_pairs_mut()
        .append_pair("client_id", &client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", SCOPE)
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent")
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", &state);
    webbrowser::open(authorization.as_str())
        .map_err(|e| format!("Couldn't open the browser for Google sign-in: {e}"))?;

    let callback = timeout(Duration::from_secs(180), async {
        loop {
            let (mut stream, _) = listener
                .accept()
                .await
                .map_err(|e| format!("Google sign-in callback failed: {e}"))?;
            let mut request = vec![0u8; 8192];
            let read = stream
                .read(&mut request)
                .await
                .map_err(|e| format!("Couldn't read Google sign-in callback: {e}"))?;
            let request = String::from_utf8_lossy(&request[..read]);
            let target = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1));
            let Some(target) = target else {
                write_callback(&mut stream, false).await?;
                continue;
            };
            let callback_url = reqwest::Url::parse(&format!("http://127.0.0.1{target}"))
                .map_err(|e| format!("Invalid Google sign-in callback: {e}"))?;
            let parameters: HashMap<_, _> = callback_url.query_pairs().into_owned().collect();
            let code = parameters.get("code");
            let oauth_error = parameters.get("error");
            if code.is_none() && oauth_error.is_none() {
                write_callback(&mut stream, false).await?;
                continue;
            }
            write_callback(&mut stream, oauth_error.is_none()).await?;
            if parameters.get("state") != Some(&state) {
                return Err("State mismatch, aborting".to_string());
            }
            if let Some(error) = oauth_error {
                return Err(format!("Google returned: {error}"));
            }
            return Ok(code.cloned().unwrap_or_default());
        }
    })
    .await
    .map_err(|_| "Timed out waiting for Google sign-in".to_string())??;

    let http = reqwest::Client::builder()
        .timeout(CLIENT_TIMEOUT)
        .build()
        .map_err(|e| format!("Couldn't create HTTP client: {e}"))?;
    let response = http
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", callback.as_str()),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
            ("code_verifier", verifier.as_str()),
        ])
        .send()
        .await
        .map_err(|e| format!("Google token exchange failed: {e}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(http_error("token exchange", status));
    }
    let token: TokenResponse = response
        .json()
        .await
        .map_err(|e| format!("Google token response was invalid: {e}"))?;
    token
        .refresh_token
        .filter(|token| !token.is_empty())
        .ok_or_else(|| "Google did not return a refresh token".to_string())
}

async fn write_callback(stream: &mut tokio::net::TcpStream, success: bool) -> Result<(), String> {
    let (status, message) = if success {
        (
            "200 OK",
            "Daylight is connected. You can close this window.",
        )
    } else {
        ("404 Not Found", "This window is only for Google sign-in.")
    };
    let body = format!(
        "<!doctype html><html><meta charset=\"utf-8\"><title>Daylight</title><body style=\"margin:0;background:#0b0b0d;color:#eeeef0;font:16px system-ui;display:grid;place-items:center;height:100vh\"><p>{message}</p></body></html>"
    );
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .await
        .map_err(|e| format!("Couldn't respond to Google sign-in: {e}"))
}

impl DriveClient {
    pub async fn new(
        client_id: &str,
        client_secret: &str,
        refresh_token: &str,
    ) -> Result<Self, String> {
        let http = reqwest::Client::builder()
            .timeout(CLIENT_TIMEOUT)
            .build()
            .map_err(|e| format!("Couldn't create HTTP client: {e}"))?;
        let response = http
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("client_id", client_id),
                ("client_secret", client_secret),
                ("refresh_token", refresh_token),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await
            .map_err(|e| format!("Google token refresh failed: {e}"))?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!(
                "Google sign-in expired ({status}). Reconnect Drive in Settings."
            ));
        }
        let token: TokenResponse = response
            .json()
            .await
            .map_err(|e| format!("Google token response was invalid: {e}"))?;
        Ok(Self {
            http,
            access_token: token.access_token,
        })
    }

    pub async fn list_files(&self) -> Result<Vec<FileMetadata>, String> {
        let mut files = Vec::new();
        let mut page_token: Option<String> = None;
        loop {
            let mut request = self
                .http
                .get(FILES_URL)
                .bearer_auth(&self.access_token)
                .query(&[
                    ("spaces", "appDataFolder"),
                    ("pageSize", "1000"),
                    ("fields", "nextPageToken,files(id,name,appProperties)"),
                    ("q", "'appDataFolder' in parents and trashed = false"),
                ]);
            if let Some(token) = &page_token {
                request = request.query(&[("pageToken", token)]);
            }
            let response = request
                .send()
                .await
                .map_err(|e| format!("Google Drive file listing failed: {e}"))?;
            let status = response.status();
            if !status.is_success() {
                return Err(http_error("file listing", status));
            }
            let page: FileList = response
                .json()
                .await
                .map_err(|e| format!("Google Drive file listing was invalid: {e}"))?;
            files.extend(page.files);
            page_token = page.next_page_token;
            if page_token.is_none() {
                return Ok(files);
            }
        }
    }

    pub async fn download(&self, file_id: &str) -> Result<Vec<u8>, String> {
        let response = self
            .http
            .get(format!("{FILES_URL}/{file_id}"))
            .bearer_auth(&self.access_token)
            .query(&[("alt", "media")])
            .send()
            .await
            .map_err(|e| format!("Google Drive download failed: {e}"))?;
        let status = response.status();
        if !status.is_success() {
            return Err(http_error("download", status));
        }
        response
            .bytes()
            .await
            .map(|bytes| bytes.to_vec())
            .map_err(|e| format!("Couldn't read Google Drive download: {e}"))
    }

    pub async fn upload(
        &self,
        existing_id: Option<&str>,
        name: &str,
        content_type: &str,
        content: &[u8],
        properties: &HashMap<String, String>,
    ) -> Result<String, String> {
        let boundary = format!("daylight-{}", uuid::Uuid::new_v4().simple());
        let mut metadata = serde_json::json!({
            "name": name,
            "appProperties": properties,
        });
        if existing_id.is_none() {
            metadata["parents"] = serde_json::json!(["appDataFolder"]);
        }
        let metadata = serde_json::to_vec(&metadata).map_err(|e| e.to_string())?;
        let mut body = Vec::with_capacity(metadata.len() + content.len() + 256);
        body.extend_from_slice(
            format!("--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n")
                .as_bytes(),
        );
        body.extend_from_slice(&metadata);
        body.extend_from_slice(
            format!("\r\n--{boundary}\r\nContent-Type: {content_type}\r\n\r\n").as_bytes(),
        );
        body.extend_from_slice(content);
        body.extend_from_slice(format!("\r\n--{boundary}--").as_bytes());
        let url = if let Some(id) = existing_id {
            format!("https://www.googleapis.com/upload/drive/v3/files/{id}")
        } else {
            "https://www.googleapis.com/upload/drive/v3/files".to_string()
        };
        let method = if existing_id.is_some() {
            reqwest::Method::PATCH
        } else {
            reqwest::Method::POST
        };
        let response = self
            .http
            .request(method, url)
            .bearer_auth(&self.access_token)
            .query(&[("uploadType", "multipart"), ("fields", "id")])
            .header(
                reqwest::header::CONTENT_TYPE,
                format!("multipart/related; boundary={boundary}"),
            )
            .body(body)
            .send()
            .await
            .map_err(|e| format!("Google Drive upload failed: {e}"))?;
        let status = response.status();
        if !status.is_success() {
            return Err(http_error("upload", status));
        }
        #[derive(Deserialize)]
        struct UploadedFile {
            id: String,
        }
        response
            .json::<UploadedFile>()
            .await
            .map(|file| file.id)
            .map_err(|e| format!("Google Drive upload response was invalid: {e}"))
    }

    pub async fn delete(&self, file_id: &str) -> Result<(), String> {
        let response = self
            .http
            .delete(format!("{FILES_URL}/{file_id}"))
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| format!("Google Drive delete failed: {e}"))?;
        if response.status().is_success() || response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(());
        }
        Err(http_error("delete", response.status()))
    }
}
