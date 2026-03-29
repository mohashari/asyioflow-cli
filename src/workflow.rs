use serde::Deserialize;
use std::collections::{HashMap, HashSet, VecDeque};
use crate::error::AppError;

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

/// Execute a validated workflow. Implemented in Task 8.
pub async fn execute_workflow(
    _workflow: &Workflow,
    _grpc: &mut crate::grpc::GrpcClient,
    _rest: &crate::rest::RestClient,
    _use_tty: bool,
    _timeout_secs: u64,
) -> Result<(), crate::error::AppError> {
    Err(crate::error::AppError::Other("workflow execution not yet implemented".to_string()))
}
