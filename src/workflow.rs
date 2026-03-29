use serde::Deserialize;
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};
use crate::error::AppError;

const POLL_INTERVAL_SECS: u64 = 2;
const TERMINAL_STATUSES: &[&str] = &["completed", "failed", "dead"];

#[derive(Debug, Deserialize, Clone)]
pub struct WorkflowStep {
    pub name: String,
    pub job_type: String,
    #[serde(default)]
    pub payload: serde_json::Value,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Workflow {
    pub name: String,
    pub steps: Vec<WorkflowStep>,
}

pub fn parse_workflow(content: &str) -> Result<Workflow, AppError> {
    serde_yaml::from_str(content)
        .map_err(|e| AppError::ValidationError(format!("invalid workflow YAML: {}", e)))
}

pub fn validate_workflow(workflow: &Workflow) -> Result<(), AppError> {
    // Duplicate name check
    let mut seen: HashSet<&str> = HashSet::new();
    for step in &workflow.steps {
        if !seen.insert(step.name.as_str()) {
            return Err(AppError::ValidationError(format!(
                "duplicate step name: {}",
                step.name
            )));
        }
    }

    // Dangling deps check
    for step in &workflow.steps {
        for dep in &step.depends_on {
            if !seen.contains(dep.as_str()) {
                return Err(AppError::ValidationError(format!(
                    "step '{}' depends on unknown step '{}'",
                    step.name, dep
                )));
            }
        }
    }

    // Cycle detection (DFS)
    let index: HashMap<&str, usize> = workflow
        .steps
        .iter()
        .enumerate()
        .map(|(i, s)| (s.name.as_str(), i))
        .collect();
    let mut visited = vec![0u8; workflow.steps.len()]; // 0=unvisited 1=in-stack 2=done

    fn dfs(
        idx: usize,
        steps: &[WorkflowStep],
        index: &HashMap<&str, usize>,
        visited: &mut Vec<u8>,
    ) -> bool {
        if visited[idx] == 1 {
            return true;
        }
        if visited[idx] == 2 {
            return false;
        }
        visited[idx] = 1;
        for dep in &steps[idx].depends_on {
            let dep_idx = *index.get(dep.as_str()).unwrap();
            if dfs(dep_idx, steps, index, visited) {
                return true;
            }
        }
        visited[idx] = 2;
        false
    }

    for i in 0..workflow.steps.len() {
        if visited[i] == 0 && dfs(i, &workflow.steps, &index, &mut visited) {
            return Err(AppError::ValidationError(
                "workflow contains a cycle".to_string(),
            ));
        }
    }

    Ok(())
}

/// Group steps into ordered batches. Each batch can run concurrently;
/// all deps of a batch are in previous batches.
pub fn topological_batches(workflow: &Workflow) -> Vec<Vec<WorkflowStep>> {
    let mut in_degree: HashMap<&str, usize> = workflow
        .steps
        .iter()
        .map(|s| (s.name.as_str(), s.depends_on.len()))
        .collect();
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();
    for step in &workflow.steps {
        for dep in &step.depends_on {
            dependents
                .entry(dep.as_str())
                .or_default()
                .push(step.name.as_str());
        }
    }
    let step_map: HashMap<&str, &WorkflowStep> = workflow
        .steps
        .iter()
        .map(|s| (s.name.as_str(), s))
        .collect();
    // Preserve YAML declaration order for deterministic output
    let step_order: HashMap<&str, usize> = workflow
        .steps
        .iter()
        .enumerate()
        .map(|(i, s)| (s.name.as_str(), i))
        .collect();

    // Seed queue in YAML declaration order
    let mut queue: VecDeque<&str> = workflow
        .steps
        .iter()
        .filter(|s| in_degree.get(s.name.as_str()).copied().unwrap_or(0) == 0)
        .map(|s| s.name.as_str())
        .collect();

    let mut batches: Vec<Vec<WorkflowStep>> = Vec::new();

    while !queue.is_empty() {
        let batch_size = queue.len();
        let mut batch: Vec<WorkflowStep> = Vec::with_capacity(batch_size);
        for _ in 0..batch_size {
            let name = queue.pop_front().unwrap();
            batch.push((*step_map[name]).clone());
        }
        let mut newly_unblocked: Vec<&str> = Vec::new();
        for step in &batch {
            for &dep in dependents.get(step.name.as_str()).unwrap_or(&vec![]) {
                let d = in_degree.get_mut(dep).unwrap();
                *d -= 1;
                if *d == 0 {
                    newly_unblocked.push(dep);
                }
            }
        }
        newly_unblocked.sort_by_key(|name| step_order.get(name).copied().unwrap_or(usize::MAX));
        for name in newly_unblocked {
            queue.push_back(name);
        }
        batches.push(batch);
    }
    batches
}

/// Execute a validated workflow against the engine.
pub async fn execute_workflow(
    workflow: &Workflow,
    grpc: &mut crate::grpc::GrpcClient,
    rest: &crate::rest::RestClient,
    _use_tty: bool,
    timeout_secs: u64,
) -> Result<(), crate::error::AppError> {
    let batches = topological_batches(workflow);
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);

    for batch in &batches {
        // Submit all steps in this batch sequentially
        let mut job_ids: Vec<(String, String)> = Vec::new(); // (step_name, job_id)
        for step in batch {
            let payload_json = step.payload.to_string();
            let resp = grpc
                .submit_job(step.job_type.clone(), payload_json, 5, 3)
                .await?;
            crate::render::print_step_update(&step.name, "submitted");
            job_ids.push((step.name.clone(), resp.id));
        }

        // Poll until all steps in batch reach terminal status
        let mut completed: HashMap<String, String> = HashMap::new(); // step_name → final_status
        loop {
            if Instant::now() >= deadline {
                return Err(crate::error::AppError::Other(format!(
                    "workflow timed out after {}s",
                    timeout_secs
                )));
            }
            tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;

            // Poll all non-terminal steps
            let jobs = rest.list_jobs(None, 1000).await.unwrap_or_default();
            for (step_name, job_id) in &job_ids {
                if completed.contains_key(step_name) {
                    continue;
                }
                if let Some(job) = jobs.iter().find(|j| &j.id == job_id) {
                    if TERMINAL_STATUSES.contains(&job.status.as_str()) {
                        crate::render::print_step_update(step_name, &job.status);
                        completed.insert(step_name.clone(), job.status.clone());
                    } else {
                        crate::render::print_step_update(step_name, &job.status);
                    }
                }
            }

            // Check for failures — cancel in-flight jobs, return error
            for (step_name, status) in &completed {
                if status == "failed" || status == "dead" {
                    for (other_step, other_job_id) in &job_ids {
                        if !completed.contains_key(other_step) {
                            let _ = grpc.cancel_job(other_job_id.clone()).await;
                        }
                    }
                    let job_id = job_ids
                        .iter()
                        .find(|(n, _)| n == step_name)
                        .map(|(_, id)| id.clone())
                        .unwrap_or_default();
                    return Err(crate::error::AppError::WorkflowFailed {
                        step: step_name.clone(),
                        job_id,
                    });
                }
            }

            // All steps in batch done?
            if completed.len() == job_ids.len() {
                break;
            }
        }
    }

    println!("workflow '{}' completed successfully", workflow.name);
    Ok(())
}
