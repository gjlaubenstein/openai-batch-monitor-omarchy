use anyhow::{bail, Context, Result};
use serde::Deserialize;

const DEFAULT_API_BASE: &str = "https://api.openai.com/v1";

#[derive(Debug, Clone, Deserialize)]
pub struct RequestCounts {
    #[serde(default)]
    pub total: u64,
    #[serde(default)]
    pub completed: u64,
    #[serde(default)]
    pub failed: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchError {
    pub code: Option<String>,
    pub message: Option<String>,
    #[allow(dead_code)]
    pub line: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchErrors {
    #[serde(default)]
    pub data: Vec<BatchError>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Batch {
    pub id: String,
    pub endpoint: String,
    pub status: String,
    #[serde(default)]
    pub input_file_id: Option<String>,
    #[serde(default)]
    pub output_file_id: Option<String>,
    #[serde(default)]
    pub error_file_id: Option<String>,
    pub created_at: i64,
    #[serde(default)]
    pub in_progress_at: Option<i64>,
    #[serde(default)]
    pub expires_at: Option<i64>,
    #[serde(default)]
    pub finalizing_at: Option<i64>,
    #[serde(default)]
    pub completed_at: Option<i64>,
    #[serde(default)]
    pub failed_at: Option<i64>,
    #[serde(default)]
    pub expired_at: Option<i64>,
    #[serde(default)]
    pub cancelling_at: Option<i64>,
    #[serde(default)]
    pub cancelled_at: Option<i64>,
    #[serde(default)]
    pub request_counts: Option<RequestCounts>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub errors: Option<BatchErrors>,
    #[serde(default)]
    pub completion_window: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BatchList {
    data: Vec<Batch>,
    #[serde(default)]
    has_more: bool,
    #[serde(default)]
    last_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorEnvelope {
    error: ApiErrorBody,
}

#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    message: String,
}

pub struct Client {
    http: reqwest::Client,
    api_key: String,
    org: Option<String>,
    project: Option<String>,
    base_url: String,
}

impl Client {
    pub fn new(
        api_key: String,
        org: Option<String>,
        project: Option<String>,
        base_url: Option<String>,
    ) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent("omarchy-batch-monitor/0.1")
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .context("failed to build HTTP client")?;
        let base_url = base_url
            .or_else(|| std::env::var("OPENAI_API_BASE").ok())
            .unwrap_or_else(|| DEFAULT_API_BASE.to_string());
        Ok(Self {
            http,
            api_key,
            org,
            project,
            base_url,
        })
    }

    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let base = self.base_url.trim_end_matches('/');
        let url = format!("{base}{path}");
        let mut req = self.http.request(method, url).bearer_auth(&self.api_key);
        if let Some(org) = &self.org {
            req = req.header("OpenAI-Organization", org);
        }
        if let Some(project) = &self.project {
            req = req.header("OpenAI-Project", project);
        }
        req
    }

    async fn check_status(resp: reqwest::Response) -> Result<reqwest::Response> {
        if resp.status().is_success() {
            return Ok(resp);
        }
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if let Ok(env) = serde_json::from_str::<ApiErrorEnvelope>(&body) {
            bail!("API error ({status}): {}", env.error.message);
        }
        bail!("API error ({status}): {body}");
    }

    /// Fetches all batches, following pagination via `after`.
    pub async fn list_all_batches(&self) -> Result<Vec<Batch>> {
        let mut all = Vec::new();
        let mut after: Option<String> = None;
        loop {
            let path = match &after {
                Some(a) => format!("/batches?limit=100&after={a}"),
                None => "/batches?limit=100".to_string(),
            };
            let resp = self.request(reqwest::Method::GET, &path).send().await?;
            let resp = Self::check_status(resp).await?;
            let list: BatchList = resp
                .json()
                .await
                .context("failed to parse batch list response")?;
            let has_more = list.has_more;
            let last_id = list.last_id.clone();
            all.extend(list.data);
            if has_more {
                if let Some(id) = last_id {
                    after = Some(id);
                    continue;
                }
            }
            break;
        }
        Ok(all)
    }

    pub async fn cancel_batch(&self, id: &str) -> Result<Batch> {
        let path = format!("/batches/{id}/cancel");
        let resp = self.request(reqwest::Method::POST, &path).send().await?;
        let resp = Self::check_status(resp).await?;
        let batch: Batch = resp
            .json()
            .await
            .context("failed to parse cancel response")?;
        Ok(batch)
    }
}
