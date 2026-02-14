use mimic::template::render_template;
use std::collections::HashMap;

#[test]
fn test_simple_variable_substitution() {
    let mut vars = HashMap::new();
    vars.insert("name".to_string(), "Alice".to_string());
    vars.insert("email".to_string(), "alice@example.com".to_string());

    let template = "Hello {{ name }}, your email is {{ email }}";
    let result = render_template(template, &vars).unwrap();

    assert_eq!(result, "Hello Alice, your email is alice@example.com");
}

#[test]
fn test_undefined_variable_error() {
    let vars = HashMap::new();

    let template = "Hello {{ name }}";
    let result = render_template(template, &vars);

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("name") || error_msg.contains("not found"),
        "Error message should mention the undefined variable 'name', got: {}",
        error_msg
    );
}

#[test]
fn test_system_variables() {
    let vars = HashMap::new();

    let template = "System: {{ hostname }} {{ os }} {{ arch }} {{ username }}";
    let result = render_template(template, &vars).unwrap();

    // All system variables should be non-empty
    assert!(result.contains("System: "));
    assert!(!result.contains("{{"));
    assert!(!result.contains("}}"));

    // Check that each system variable was replaced with something
    let parts: Vec<&str> = result.split_whitespace().collect();
    assert!(
        parts.len() >= 5,
        "Expected at least 5 parts (System: + 4 variables)"
    );
}

#[test]
fn test_user_variables_override_system() {
    let mut vars = HashMap::new();
    vars.insert("hostname".to_string(), "custom-host".to_string());

    let template = "Host: {{ hostname }}";
    let result = render_template(template, &vars).unwrap();

    assert_eq!(result, "Host: custom-host");
}

#[test]
fn test_mixed_system_and_user_variables() {
    let mut vars = HashMap::new();
    vars.insert("app_name".to_string(), "MyApp".to_string());

    let template = "{{ app_name }} running on {{ hostname }} ({{ os }})";
    let result = render_template(template, &vars).unwrap();

    assert!(result.starts_with("MyApp running on "));
    assert!(!result.contains("{{"));
    assert!(!result.contains("}}"));
}

#[test]
fn test_empty_template() {
    let vars = HashMap::new();

    let template = "";
    let result = render_template(template, &vars).unwrap();

    assert_eq!(result, "");
}

#[test]
fn test_no_variables_in_template() {
    let vars = HashMap::new();

    let template = "This is plain text with no variables";
    let result = render_template(template, &vars).unwrap();

    assert_eq!(result, "This is plain text with no variables");
}
