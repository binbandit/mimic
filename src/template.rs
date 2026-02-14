use anyhow::Result;
use handlebars::Handlebars;
use std::collections::HashMap;

pub fn render_template(template: &str, variables: &HashMap<String, String>) -> Result<String> {
    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(true);

    let mut context = get_system_variables();

    for (key, value) in variables {
        context.insert(key.clone(), value.clone());
    }

    handlebars.render_template(template, &context).map_err(|e| {
        anyhow::anyhow!(
            "Variable '{}' not found in context",
            extract_variable_name(&e.to_string())
        )
    })
}

fn get_system_variables() -> HashMap<String, String> {
    let mut vars = HashMap::new();

    vars.insert(
        "hostname".to_string(),
        whoami::hostname().unwrap_or_else(|_| "unknown".to_string()),
    );
    vars.insert(
        "username".to_string(),
        whoami::username().unwrap_or_else(|_| "unknown".to_string()),
    );
    vars.insert("os".to_string(), std::env::consts::OS.to_string());
    vars.insert("arch".to_string(), std::env::consts::ARCH.to_string());

    vars
}

fn extract_variable_name(error_msg: &str) -> &str {
    if let Some(start) = error_msg.find('"') {
        if let Some(end) = error_msg[start + 1..].find('"') {
            return &error_msg[start + 1..start + 1 + end];
        }
    }
    "unknown"
}
