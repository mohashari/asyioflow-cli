use comfy_table::Table;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::HashMap;
use crate::rest::{Job, StatusSummary};
use crate::grpc::JobResponse;

// --- Metric types and parsing ---

#[derive(Default)]
pub struct MetricValues {
    pub submitted: u64,
    pub completed: u64,
    pub failed: u64,
    pub queue_depth: u64,
}

pub fn parse_metrics(text: &str) -> MetricValues {
    let mut m = MetricValues::default();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        if parts.len() != 2 {
            continue;
        }
        let val: f64 = parts[1].parse().unwrap_or(0.0);
        match parts[0] {
            "asyioflow_jobs_submitted_total" => m.submitted = val as u64,
            "asyioflow_jobs_completed_total" => m.completed = val as u64,
            "asyioflow_jobs_failed_total" => m.failed = val as u64,
            "asyioflow_queue_depth" => m.queue_depth = val as u64,
            _ => {}
        }
    }
    m
}

pub fn metrics_to_json(m: &MetricValues) -> String {
    let obj = serde_json::json!({
        "jobs_submitted_total": m.submitted,
        "jobs_completed_total": m.completed,
        "jobs_failed_total": m.failed,
        "queue_depth": m.queue_depth,
    });
    serde_json::to_string_pretty(&obj).unwrap()
}

// --- Job output ---

pub fn print_job_id(id: &str) {
    println!("job-id: {}", id);
}

pub fn print_cancelled(id: &str) {
    println!("cancelled: {}", id);
}

pub fn print_job_detail(job: &JobResponse) {
    let mut table = Table::new();
    table.set_header(vec!["Field", "Value"]);
    table.add_row(vec!["id", &job.id]);
    table.add_row(vec!["type", &job.job_type]);
    table.add_row(vec!["status", &job.status]);
    table.add_row(vec!["attempts", &job.attempts.to_string()]);
    table.add_row(vec!["max_retries", &job.max_retries.to_string()]);
    table.add_row(vec!["created_at", &job.created_at]);
    table.add_row(vec!["updated_at", &job.updated_at]);
    if !job.error.is_empty() {
        table.add_row(vec!["error", &job.error]);
    }
    println!("{}", table);
}

pub fn print_jobs_table(jobs: &[Job]) {
    if jobs.is_empty() {
        println!("no jobs found");
        return;
    }
    let mut table = Table::new();
    table.set_header(vec!["id", "type", "status", "priority", "attempts", "created_at"]);
    for job in jobs {
        table.add_row(vec![
            &job.id,
            &job.job_type,
            &job.status,
            &job.priority.to_string(),
            &job.attempts.to_string(),
            &job.created_at,
        ]);
    }
    println!("{}", table);
}

pub fn print_jobs_json(jobs: &[Job]) {
    println!("{}", serde_json::to_string_pretty(jobs).unwrap());
}

// --- Status output ---

pub fn print_status_table(s: &StatusSummary) {
    let mut table = Table::new();
    table.set_header(vec!["Field", "Value"]);
    let reach = if s.rest_reachable { "✓" } else { "✗" };
    table.add_row(vec!["gRPC address", &s.grpc_addr]);
    table.add_row(vec!["REST address", &s.rest_addr]);
    table.add_row(vec!["reachable", reach]);
    table.add_row(vec!["total", &s.total.to_string()]);
    table.add_row(vec!["pending", &s.pending.to_string()]);
    table.add_row(vec!["queued", &s.queued.to_string()]);
    table.add_row(vec!["running", &s.running.to_string()]);
    table.add_row(vec!["completed", &s.completed.to_string()]);
    table.add_row(vec!["failed", &s.failed.to_string()]);
    table.add_row(vec!["dead", &s.dead.to_string()]);
    println!("{}", table);
}

// --- Metrics output ---

pub fn print_metrics_table(m: &MetricValues) {
    let mut table = Table::new();
    table.set_header(vec!["Metric", "Value"]);
    table.add_row(vec!["jobs_submitted_total", &m.submitted.to_string()]);
    table.add_row(vec!["jobs_completed_total", &m.completed.to_string()]);
    table.add_row(vec!["jobs_failed_total", &m.failed.to_string()]);
    table.add_row(vec!["queue_depth", &m.queue_depth.to_string()]);
    println!("{}", table);
}

pub fn print_metrics_json(m: &MetricValues) {
    println!("{}", metrics_to_json(m));
}

// --- Workflow progress (static / CI mode) ---

pub fn print_step_update(step: &str, status: &str) {
    println!("[{}] {}", step, status);
}

/// Returns a MultiProgress and a map of step_name → ProgressBar.
pub fn create_workflow_progress(step_names: &[&str]) -> (MultiProgress, HashMap<String, ProgressBar>) {
    let mp = MultiProgress::new();
    let style = ProgressStyle::with_template("{spinner:.green} {prefix:.bold} {msg}")
        .unwrap()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");
    let mut bars = HashMap::new();
    for name in step_names {
        let pb = mp.add(ProgressBar::new_spinner());
        pb.set_style(style.clone());
        pb.set_prefix(name.to_string());
        pb.set_message("waiting");
        bars.insert(name.to_string(), pb);
    }
    (mp, bars)
}

pub fn update_step_progress(bars: &HashMap<String, ProgressBar>, step: &str, status: &str) {
    if let Some(pb) = bars.get(step) {
        pb.set_message(status.to_string());
        pb.tick();
        if status == "completed" || status == "dead" || status == "failed" {
            pb.finish();
        }
    }
}
