use mimic::config::should_apply_for_roles;

#[test]
fn test_only_roles_matching() {
    let only = Some(vec!["work".to_string()]);
    let skip = None;
    let host_roles = vec!["work".to_string(), "mac".to_string()];

    assert!(should_apply_for_roles(&only, &skip, &host_roles));
}

#[test]
fn test_only_roles_not_matching() {
    let only = Some(vec!["work".to_string()]);
    let skip = None;
    let host_roles = vec!["personal".to_string()];

    assert!(!should_apply_for_roles(&only, &skip, &host_roles));
}

#[test]
fn test_skip_roles_matching() {
    let only = None;
    let skip = Some(vec!["work".to_string()]);
    let host_roles = vec!["work".to_string()];

    assert!(!should_apply_for_roles(&only, &skip, &host_roles));
}

#[test]
fn test_skip_roles_not_matching() {
    let only = None;
    let skip = Some(vec!["work".to_string()]);
    let host_roles = vec!["personal".to_string()];

    assert!(should_apply_for_roles(&only, &skip, &host_roles));
}

#[test]
fn test_both_set_only_wins() {
    let only = Some(vec!["work".to_string()]);
    let skip = Some(vec!["server".to_string()]);
    let host_roles = vec!["work".to_string(), "mac".to_string()];

    assert!(should_apply_for_roles(&only, &skip, &host_roles));
}

#[test]
fn test_both_set_skip_wins() {
    let only = Some(vec!["work".to_string()]);
    let skip = Some(vec!["server".to_string()]);
    let host_roles = vec!["work".to_string(), "server".to_string()];

    assert!(!should_apply_for_roles(&only, &skip, &host_roles));
}

#[test]
fn test_no_restrictions_applies() {
    let only = None;
    let skip = None;
    let host_roles = vec!["anything".to_string()];

    assert!(should_apply_for_roles(&only, &skip, &host_roles));
}

#[test]
fn test_empty_only_roles_applies_to_all() {
    let only = Some(vec![]);
    let skip = None;
    let host_roles = vec!["work".to_string()];

    assert!(should_apply_for_roles(&only, &skip, &host_roles));
}

#[test]
fn test_multiple_only_roles_any_match() {
    let only = Some(vec!["desktop".to_string(), "powerful".to_string()]);
    let skip = None;
    let host_roles = vec!["desktop".to_string()];

    assert!(should_apply_for_roles(&only, &skip, &host_roles));
}

#[test]
fn test_multiple_skip_roles_any_match() {
    let only = None;
    let skip = Some(vec!["server".to_string(), "headless".to_string()]);
    let host_roles = vec!["headless".to_string()];

    assert!(!should_apply_for_roles(&only, &skip, &host_roles));
}

#[test]
fn test_empty_host_roles_with_only_roles() {
    let only = Some(vec!["work".to_string()]);
    let skip = None;
    let host_roles: Vec<String> = vec![];

    assert!(!should_apply_for_roles(&only, &skip, &host_roles));
}

#[test]
fn test_empty_host_roles_with_skip_roles() {
    let only = None;
    let skip = Some(vec!["work".to_string()]);
    let host_roles: Vec<String> = vec![];

    assert!(should_apply_for_roles(&only, &skip, &host_roles));
}
