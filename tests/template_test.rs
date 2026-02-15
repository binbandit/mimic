use mimic::config::Dotfile;
use mimic::template::{render_template, render_template_with_host, HostContext};
use std::collections::HashMap;

#[test]
fn test_simple_variable_substitution() {
    let mut vars = HashMap::new();
    vars.insert("name".to_string(), "Alice".to_string());
    vars.insert("email".to_string(), "alice@example.com".to_string());

    let template = "Hello {{ variables.name }}, your email is {{ variables.email }}";
    let result = render_template(template, &vars).unwrap();

    assert_eq!(result, "Hello Alice, your email is alice@example.com");
}

#[test]
fn test_undefined_variable_error() {
    let vars = HashMap::new();

    let template = "Hello {{ variables.name }}";
    let result = render_template(template, &vars);

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("name")
            || error_msg.contains("not found")
            || error_msg.contains("rendering"),
        "Error message should mention the undefined variable 'name', got: {}",
        error_msg
    );
}

#[test]
fn test_system_variables() {
    let vars = HashMap::new();

    let template =
        "System: {{ system.hostname }} {{ system.os }} {{ system.arch }} {{ system.username }}";
    let result = render_template(template, &vars).unwrap();

    assert!(result.contains("System: "));
    assert!(!result.contains("{{"));
    assert!(!result.contains("}}"));

    let parts: Vec<&str> = result.split_whitespace().collect();
    assert!(
        parts.len() >= 5,
        "Expected at least 5 parts (System: + 4 variables)"
    );
}

#[test]
fn test_mixed_system_and_user_variables() {
    let mut vars = HashMap::new();
    vars.insert("app_name".to_string(), "MyApp".to_string());

    let template = "{{ variables.app_name }} running on {{ system.hostname }} ({{ system.os }})";
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

#[test]
fn test_template_detection_tmpl_suffix() {
    let dotfile = Dotfile {
        source: "config.fish.tmpl".to_string(),
        target: "~/.config/fish/config.fish".to_string(),
        template: false,
    };
    assert!(dotfile.is_template());
}

#[test]
fn test_template_detection_hbs_suffix() {
    let dotfile = Dotfile {
        source: "config.fish.hbs".to_string(),
        target: "~/.config/fish/config.fish".to_string(),
        template: false,
    };
    assert!(dotfile.is_template());
}

#[test]
fn test_template_detection_explicit_flag() {
    let dotfile = Dotfile {
        source: "config.fish".to_string(),
        target: "~/.config/fish/config.fish".to_string(),
        template: true,
    };
    assert!(dotfile.is_template());
}

#[test]
fn test_template_detection_regular_file() {
    let dotfile = Dotfile {
        source: "config.fish".to_string(),
        target: "~/.config/fish/config.fish".to_string(),
        template: false,
    };
    assert!(!dotfile.is_template());
}

#[test]
fn test_host_context_name() {
    let vars = HashMap::new();
    let host_ctx = HostContext {
        name: "work-laptop".to_string(),
        roles: vec![],
    };

    let template = "Host: {{ host.name }}";
    let result = render_template_with_host(template, &vars, &host_ctx).unwrap();

    assert_eq!(result, "Host: work-laptop");
}

#[test]
fn test_host_context_roles_with_includes() {
    let vars = HashMap::new();
    let host_ctx = HostContext {
        name: "work-laptop".to_string(),
        roles: vec!["work".to_string(), "development".to_string()],
    };

    let template = r#"{{#if (includes host.roles "work")}}WORK{{/if}}"#;
    let result = render_template_with_host(template, &vars, &host_ctx).unwrap();

    assert_eq!(result, "WORK");
}

#[test]
fn test_includes_helper_not_in_array() {
    let vars = HashMap::new();
    let host_ctx = HostContext {
        name: "personal-laptop".to_string(),
        roles: vec!["personal".to_string()],
    };

    let template = r#"{{#if (includes host.roles "work")}}WORK{{/if}}{{#if (includes host.roles "personal")}}PERSONAL{{/if}}"#;
    let result = render_template_with_host(template, &vars, &host_ctx).unwrap();

    assert_eq!(result, "PERSONAL");
}

#[test]
fn test_includes_helper_empty_array() {
    let vars = HashMap::new();
    let host_ctx = HostContext {
        name: "test".to_string(),
        roles: vec![],
    };

    let template = r#"{{#if (includes host.roles "work")}}WORK{{else}}NO_WORK{{/if}}"#;
    let result = render_template_with_host(template, &vars, &host_ctx).unwrap();

    assert_eq!(result, "NO_WORK");
}

#[test]
fn test_complex_template_with_all_features() {
    let mut vars = HashMap::new();
    vars.insert("editor".to_string(), "nvim".to_string());
    vars.insert(
        "http_proxy".to_string(),
        "http://localhost:3128".to_string(),
    );

    let host_ctx = HostContext {
        name: "work-laptop".to_string(),
        roles: vec!["work".to_string()],
    };

    let template = r#"# Config for {{ host.name }}
# User: {{ system.username }}@{{ system.hostname }}

set -gx EDITOR {{ variables.editor }}

{{#if (includes host.roles "work")}}
set -gx HTTP_PROXY "{{ variables.http_proxy }}"
{{/if}}
"#;

    let result = render_template_with_host(template, &vars, &host_ctx).unwrap();

    assert!(result.contains("# Config for work-laptop"));
    assert!(result.contains("set -gx EDITOR nvim"));
    assert!(result.contains("set -gx HTTP_PROXY \"http://localhost:3128\""));
}

#[test]
fn test_variables_namespace() {
    let mut vars = HashMap::new();
    vars.insert("editor".to_string(), "vim".to_string());

    let template = "Editor: {{ variables.editor }}";
    let result = render_template(template, &vars).unwrap();

    assert_eq!(result, "Editor: vim");
}

#[test]
fn test_system_namespace() {
    let vars = HashMap::new();

    let template = "OS: {{ system.os }}, Arch: {{ system.arch }}";
    let result = render_template(template, &vars).unwrap();

    assert!(result.starts_with("OS: "));
    assert!(result.contains(", Arch: "));
}
