use std::fmt;

#[derive(Debug)]
pub enum AppError {
    EngineUnreachable(String),
    NotFound(String),
    ValidationError(String),
    WorkflowFailed { step: String, job_id: String },
    Other(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::EngineUnreachable(msg) => write!(f, "engine unreachable: {}", msg),
            AppError::NotFound(msg) => write!(f, "not found: {}", msg),
            AppError::ValidationError(msg) => write!(f, "validation error: {}", msg),
            AppError::WorkflowFailed { step, job_id } => {
                write!(f, "workflow step '{}' failed (job: {})", step, job_id)
            }
            AppError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl From<tonic::Status> for AppError {
    fn from(status: tonic::Status) -> Self {
        match status.code() {
            tonic::Code::NotFound => AppError::NotFound(status.message().to_string()),
            tonic::Code::Unavailable => {
                AppError::EngineUnreachable(status.message().to_string())
            }
            _ => AppError::Other(status.message().to_string()),
        }
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_connect() || e.is_timeout() {
            AppError::EngineUnreachable(e.to_string())
        } else {
            AppError::Other(e.to_string())
        }
    }
}

/// Maps AppError variant to process exit code.
/// 0=success (not an error), 1=logical failure, 2=engine unreachable, 3=bad args (Clap handles automatically)
pub fn exit_code(e: &AppError) -> i32 {
    match e {
        AppError::EngineUnreachable(_) => 2,
        _ => 1,
    }
}
