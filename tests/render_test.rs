use asyioflow_cli::render::{parse_metrics, MetricValues};

#[test]
fn test_parse_metrics_all_fields() {
    let input = "
# HELP asyioflow_jobs_submitted_total Total jobs submitted
# TYPE asyioflow_jobs_submitted_total counter
asyioflow_jobs_submitted_total 100
asyioflow_jobs_completed_total 95
asyioflow_jobs_failed_total 3
asyioflow_queue_depth 2
";
    let m = parse_metrics(input);
    assert_eq!(m.submitted, 100);
    assert_eq!(m.completed, 95);
    assert_eq!(m.failed, 3);
    assert_eq!(m.queue_depth, 2);
}

#[test]
fn test_parse_metrics_skips_comments_and_blanks() {
    let input = "# TYPE foo counter\n\nasyioflow_queue_depth 7\n";
    let m = parse_metrics(input);
    assert_eq!(m.queue_depth, 7);
    assert_eq!(m.submitted, 0);
}

#[test]
fn test_metrics_json_output() {
    use asyioflow_cli::render::metrics_to_json;
    let m = MetricValues {
        submitted: 10,
        completed: 8,
        failed: 1,
        queue_depth: 3,
    };
    let json = metrics_to_json(&m);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["jobs_submitted_total"], 10);
    assert_eq!(v["jobs_failed_total"], 1);
}
