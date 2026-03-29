mod config;
mod error;
mod grpc;
mod render;
mod rest;
mod workflow;

use clap::{Parser, Subcommand, ValueEnum};
use config::Config;
use error::{exit_code, AppError};
use std::process;

#[derive(Parser)]
#[command(
    name = "asyioflow",
    version,
    about = "CLI for interacting with the AysioFlow engine"
)]
struct Cli {
    /// gRPC server address (host:port)
    #[arg(long, global = true, env = "ASYIOFLOW_GRPC_ADDR")]
    grpc_addr: Option<String>,

    /// REST server address (http://host:port)
    #[arg(long, global = true, env = "ASYIOFLOW_REST_ADDR")]
    rest_addr: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Job management
    Job {
        #[command(subcommand)]
        cmd: JobCommands,
    },
    /// Run a workflow from a YAML/JSON file
    Workflow {
        #[command(subcommand)]
        cmd: WorkflowCommands,
    },
    /// Check engine reachability and job counts
    Status,
    /// Show Prometheus metrics
    Metrics {
        #[arg(long, value_enum, default_value = "table")]
        output: OutputFormat,
    },
}

#[derive(Subcommand)]
enum JobCommands {
    /// Submit a new job
    Submit {
        #[arg(long, name = "type")]
        job_type: String,
        /// JSON payload string
        #[arg(long, default_value = "{}")]
        payload: String,
        /// Priority: 1 (low), 5 (normal), 10 (high)
        #[arg(long, default_value_t = 5)]
        priority: i32,
        #[arg(long, default_value_t = 3)]
        max_retries: i32,
    },
    /// Get job details by ID
    Get { id: String },
    /// Cancel a job by ID
    Cancel { id: String },
    /// List jobs
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: u32,
        #[arg(long, value_enum, default_value = "table")]
        output: OutputFormat,
    },
}

#[derive(Subcommand)]
enum WorkflowCommands {
    /// Run a workflow file
    Run {
        file: std::path::PathBuf,
        /// Timeout in seconds
        #[arg(long, default_value_t = 600)]
        timeout: u64,
        /// Disable live TUI (use static output for CI)
        #[arg(long)]
        no_tty: bool,
    },
}

#[derive(ValueEnum, Clone)]
enum OutputFormat {
    Table,
    Json,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let config = Config::new(cli.grpc_addr, cli.rest_addr);
    if let Err(e) = run(cli.command, config).await {
        eprintln!("error: {}", e);
        process::exit(exit_code(&e));
    }
}

async fn run(command: Commands, config: Config) -> Result<(), AppError> {
    match command {
        Commands::Job { cmd } => run_job(cmd, &config).await,
        Commands::Workflow { cmd } => run_workflow_cmd(cmd, &config).await,
        Commands::Status => run_status(&config).await,
        Commands::Metrics { output } => run_metrics(output, &config).await,
    }
}

async fn run_job(cmd: JobCommands, config: &Config) -> Result<(), AppError> {
    match cmd {
        JobCommands::Submit {
            job_type,
            payload,
            priority,
            max_retries,
        } => {
            let mut grpc = grpc::GrpcClient::connect(&config.grpc_addr).await?;
            let resp = grpc
                .submit_job(job_type, payload, priority, max_retries)
                .await?;
            render::print_job_id(&resp.id);
        }
        JobCommands::Get { id } => {
            let mut grpc = grpc::GrpcClient::connect(&config.grpc_addr).await?;
            let resp = grpc.get_job(id).await?;
            render::print_job_detail(&resp);
        }
        JobCommands::Cancel { id } => {
            let mut grpc = grpc::GrpcClient::connect(&config.grpc_addr).await?;
            let success = grpc.cancel_job(id.clone()).await?;
            if success {
                render::print_cancelled(&id);
            } else {
                return Err(AppError::Other(format!("cancel returned false for {}", id)));
            }
        }
        JobCommands::List {
            status,
            limit,
            output,
        } => {
            let rest = rest::RestClient::new(&config.rest_addr);
            let jobs = rest.list_jobs(status.as_deref(), limit).await?;
            match output {
                OutputFormat::Table => render::print_jobs_table(&jobs),
                OutputFormat::Json => render::print_jobs_json(&jobs),
            }
        }
    }
    Ok(())
}

async fn run_status(config: &Config) -> Result<(), AppError> {
    let rest = rest::RestClient::new(&config.rest_addr);
    let summary = rest.get_status(&config.grpc_addr).await;
    render::print_status_table(&summary);
    Ok(())
}

async fn run_metrics(output: OutputFormat, config: &Config) -> Result<(), AppError> {
    let rest = rest::RestClient::new(&config.rest_addr);
    let text = rest.get_metrics().await?;
    let metrics = render::parse_metrics(&text);
    match output {
        OutputFormat::Table => render::print_metrics_table(&metrics),
        OutputFormat::Json => render::print_metrics_json(&metrics),
    }
    Ok(())
}

async fn run_workflow_cmd(cmd: WorkflowCommands, config: &Config) -> Result<(), AppError> {
    match cmd {
        WorkflowCommands::Run { file, timeout, no_tty } => {
            let content = std::fs::read_to_string(&file)
                .map_err(|e| AppError::ValidationError(format!("cannot read {:?}: {}", file, e)))?;
            let wf = workflow::parse_workflow(&content)?;
            workflow::validate_workflow(&wf)?;
            let mut grpc = grpc::GrpcClient::connect(&config.grpc_addr).await?;
            let rest = rest::RestClient::new(&config.rest_addr);
            let use_tty = !no_tty && is_terminal::is_terminal(std::io::stdout());
            workflow::execute_workflow(&wf, &mut grpc, &rest, use_tty, timeout).await
        }
    }
}
