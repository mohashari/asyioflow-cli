use asyioflow_cli::config::Config;
use std::sync::Mutex;

// Serialize all tests that touch env vars — Cargo runs tests in parallel threads.
static ENV_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn test_defaults_when_no_env_no_flags() {
    let _lock = ENV_MUTEX.lock().unwrap();
    std::env::remove_var("ASYIOFLOW_GRPC_ADDR");
    std::env::remove_var("ASYIOFLOW_REST_ADDR");
    let cfg = Config::new(None, None);
    assert_eq!(cfg.grpc_addr, "localhost:9090");
    assert_eq!(cfg.rest_addr, "http://localhost:8080");
}

#[test]
fn test_env_var_overrides_default() {
    let _lock = ENV_MUTEX.lock().unwrap();
    std::env::set_var("ASYIOFLOW_GRPC_ADDR", "remote:9090");
    std::env::set_var("ASYIOFLOW_REST_ADDR", "http://remote:8080");
    let cfg = Config::new(None, None);
    assert_eq!(cfg.grpc_addr, "remote:9090");
    assert_eq!(cfg.rest_addr, "http://remote:8080");
    std::env::remove_var("ASYIOFLOW_GRPC_ADDR");
    std::env::remove_var("ASYIOFLOW_REST_ADDR");
}

#[test]
fn test_flag_overrides_env_var() {
    let _lock = ENV_MUTEX.lock().unwrap();
    std::env::set_var("ASYIOFLOW_GRPC_ADDR", "remote:9090");
    let cfg = Config::new(Some("flag-host:9090".to_string()), None);
    assert_eq!(cfg.grpc_addr, "flag-host:9090");
    // rest_addr falls back to default since REST env var was not set
    assert_eq!(cfg.rest_addr, "http://localhost:8080");
    std::env::remove_var("ASYIOFLOW_GRPC_ADDR");
}
