#[derive(Debug, Clone)]
pub struct Config {
    pub grpc_addr: String,
    pub rest_addr: String,
}

impl Config {
    /// Resolves config with priority: CLI flag > env var > default.
    pub fn new(grpc_flag: Option<String>, rest_flag: Option<String>) -> Self {
        let grpc_addr = grpc_flag
            .or_else(|| std::env::var("ASYIOFLOW_GRPC_ADDR").ok())
            .unwrap_or_else(|| "localhost:9090".to_string());
        let rest_addr = rest_flag
            .or_else(|| std::env::var("ASYIOFLOW_REST_ADDR").ok())
            .unwrap_or_else(|| "http://localhost:8080".to_string());
        Config { grpc_addr, rest_addr }
    }
}
