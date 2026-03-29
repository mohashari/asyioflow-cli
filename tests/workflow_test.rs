use asyioflow_cli::workflow::{parse_workflow, validate_workflow, topological_batches};

fn simple_yaml() -> &'static str {
    r#"
name: test-pipeline
steps:
  - name: fetch
    job_type: http-fetch
    payload: {}
    depends_on: []
  - name: transform
    job_type: data-transform
    payload: {}
    depends_on: [fetch]
  - name: load
    job_type: db-load
    payload: {}
    depends_on: [transform]
"#
}

#[test]
fn test_parse_valid_yaml() {
    let wf = parse_workflow(simple_yaml()).unwrap();
    assert_eq!(wf.name, "test-pipeline");
    assert_eq!(wf.steps.len(), 3);
    assert_eq!(wf.steps[1].depends_on, vec!["fetch"]);
}

#[test]
fn test_validate_ok() {
    let wf = parse_workflow(simple_yaml()).unwrap();
    assert!(validate_workflow(&wf).is_ok());
}

#[test]
fn test_validate_duplicate_step_name() {
    let yaml = r#"
name: dup
steps:
  - name: a
    job_type: t
    depends_on: []
  - name: a
    job_type: t
    depends_on: []
"#;
    let wf = parse_workflow(yaml).unwrap();
    let err = validate_workflow(&wf).unwrap_err();
    assert!(err.to_string().contains("duplicate"));
}

#[test]
fn test_validate_dangling_dep() {
    let yaml = r#"
name: dangling
steps:
  - name: a
    job_type: t
    depends_on: [nonexistent]
"#;
    let wf = parse_workflow(yaml).unwrap();
    let err = validate_workflow(&wf).unwrap_err();
    assert!(err.to_string().contains("unknown step"));
}

#[test]
fn test_validate_cycle_detection() {
    let yaml = r#"
name: cycle
steps:
  - name: a
    job_type: t
    depends_on: [b]
  - name: b
    job_type: t
    depends_on: [a]
"#;
    let wf = parse_workflow(yaml).unwrap();
    let err = validate_workflow(&wf).unwrap_err();
    assert!(err.to_string().contains("cycle"));
}

#[test]
fn test_topological_batches_linear() {
    let wf = parse_workflow(simple_yaml()).unwrap();
    let batches = topological_batches(&wf);
    assert_eq!(batches.len(), 3);
    assert_eq!(batches[0].len(), 1);
    assert_eq!(batches[0][0].name, "fetch");
    assert_eq!(batches[1][0].name, "transform");
    assert_eq!(batches[2][0].name, "load");
}

#[test]
fn test_topological_batches_parallel() {
    let yaml = r#"
name: parallel
steps:
  - name: a
    job_type: t
    depends_on: []
  - name: b
    job_type: t
    depends_on: []
  - name: c
    job_type: t
    depends_on: [a, b]
"#;
    let wf = parse_workflow(yaml).unwrap();
    let batches = topological_batches(&wf);
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0].len(), 2); // a and b run in parallel
    assert_eq!(batches[1].len(), 1); // c waits for both
    assert_eq!(batches[1][0].name, "c");
}
