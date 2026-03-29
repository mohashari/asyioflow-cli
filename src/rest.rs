use serde::Deserialize;
use crate::error::AppError;

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct Job {
    pub id: String,
    pub job_type: String,
    pub status: String,
    pub priority: i32,
    pub attempts: i32,
    pub max_retries: i32,
    pub error: Option<String>,
    pub created_at: String,
}

#[derive(Debug)]
pub struct StatusSummary {
    pub grpc_addr: String,
    pub rest_addr: String,
    pub rest_reachable: bool,
    pub total: usize,
    pub pending: usize,
    pub queued: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub dead: usize,
}

pub struct RestClient {
    client: reqwest::Client,
    pub base_url: String,
}

impl RestClient {
    pub fn new(base_url: &str) -> Self {
        RestClient {
            client: reqwest::Client::new(),
            base_url: base_url.to_string(),
        }
    }

    pub async fn list_jobs(
        &self,
        status_filter: Option<&str>,
        limit: u32,
    ) -> Result<Vec<Job>, AppError> {
        let mut url = format!("{}/api/v1/jobs?limit={}", self.base_url, limit);
        if let Some(s) = status_filter {
            url.push_str(&format!("&status={}", s));
        }
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Err(AppError::Other(format!("HTTP {}", resp.status())));
        }
        resp.json::<Vec<Job>>().await.map_err(AppError::from)
    }

    pub async fn get_job(&self, id: &str) -> Result<Job, AppError> {
        let url = format!("{}/api/v1/jobs/{}", self.base_url, id);
        let resp = self.client.get(&url).send().await?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::NotFound(id.to_string()));
        }
        if !resp.status().is_success() {
            return Err(AppError::Other(format!("HTTP {}", resp.status())));
        }
        resp.json::<Job>().await.map_err(AppError::from)
    }

    pub async fn get_metrics(&self) -> Result<String, AppError> {
        let url = format!("{}/metrics", self.base_url);
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Err(AppError::Other(format!("HTTP {}", resp.status())));
        }
        resp.text().await.map_err(AppError::from)
    }

    pub async fn get_status(&self, grpc_addr: &str) -> StatusSummary {
        let url = format!("{}/api/v1/jobs?limit=10000", self.base_url);
        let (rest_reachable, jobs) = match self.client.get(&url).send().await {
            Ok(resp) => {
                let jobs = resp.json::<Vec<Job>>().await.unwrap_or_default();
                (true, jobs)
            }
            Err(_) => (false, vec![]),
        };

        let mut summary = StatusSummary {
            grpc_addr: grpc_addr.to_string(),
            rest_addr: self.base_url.clone(),
            rest_reachable,
            total: jobs.len(),
            pending: 0,
            queued: 0,
            running: 0,
            completed: 0,
            failed: 0,
            dead: 0,
        };
        for job in &jobs {
            match job.status.as_str() {
                "pending" => summary.pending += 1,
                "queued" => summary.queued += 1,
                "running" => summary.running += 1,
                "completed" => summary.completed += 1,
                "failed" => summary.failed += 1,
                "dead" => summary.dead += 1,
                _ => {}
            }
        }
        summary
    }
}
