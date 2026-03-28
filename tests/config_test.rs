use asyioflow_cli::config::Config;

#[test]
fn test_defaults_when_no_env_no_flags() {
    // Clear env vars in case they're set
    std::env::remove_var("ASYIOFLOW_GRPC_ADDR");
    std::env::remove_var("ASYIOFLOW_REST_ADDR");
    let cfg = Config::new(None, None);
    assert_eq!(cfg.grpc_addr, "localhost:9090");
    assert_eq!(cfg.rest_addr, "http://localhost:8080");
}

#[test]
fn test_env_var_overrides_default() {
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
    std::env::set_var("ASYIOFLOW_GRPC_ADDR", "remote:9090");
    let cfg = Config::new(Some("flag-host:9090".to_string()), None);
    assert_eq!(cfg.grpc_addr, "flag-host:9090");
    std::env::remove_var("ASYIOFLOW_GRPC_ADDR");
}
