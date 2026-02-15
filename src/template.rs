use anyhow::{Context, Result};
use handlebars::{handlebars_helper, Handlebars};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;

handlebars_helper!(includes: |array: Vec<Value>, value: Value| {
    array.iter().any(|v| v == &value)
});

#[derive(Debug, Clone)]
pub struct HostContext {
    pub name: String,
    pub roles: Vec<String>,
}

pub fn render_template(template: &str, variables: &HashMap<String, String>) -> Result<String> {
    let host_context = HostContext {
        name: "default".to_string(),
        roles: vec![],
    };
    render_template_with_host(template, variables, &host_context)
}

pub fn render_template_with_host(
    template: &str,
    variables: &HashMap<String, String>,
    host_context: &HostContext,
) -> Result<String> {
    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(true);

    handlebars.register_helper("includes", Box::new(includes));

    let context = json!({
        "variables": variables,
        "host": {
            "name": host_context.name,
            "roles": host_context.roles,
        },
        "system": {
            "hostname": whoami::hostname().unwrap_or_else(|_| "unknown".to_string()),
            "username": whoami::username().unwrap_or_else(|_| "unknown".to_string()),
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
        },
    });

    handlebars
        .render_template(template, &context)
        .map_err(|e| anyhow::anyhow!("Template rendering failed: {}", e))
}

pub fn render_file(
    source_path: &Path,
    variables: &HashMap<String, String>,
    host_context: &HostContext,
) -> Result<String> {
    let content = std::fs::read_to_string(source_path)
        .with_context(|| format!("Failed to read template: {}", source_path.display()))?;

    render_template_with_host(&content, variables, host_context)
        .with_context(|| format!("Template rendering failed for: {}", source_path.display()))
}
