use crate::error::{AppError, AppResult};
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedMessageTemplate {
    pub rendered: String,
    pub variables: Vec<String>,
}

pub fn render_template_body(body: &str, context: &Value) -> AppResult<RenderedMessageTemplate> {
    let tokens = extract_variable_tokens(body);
    let variables = tokens
        .iter()
        .map(|(_, variable)| variable.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut rendered = body.to_string();

    for (token, variable) in &tokens {
        let value = lookup_context_value(context, variable).ok_or_else(|| {
            AppError::Validation(format!("Unknown template variable: {variable}"))
        })?;
        rendered = rendered.replace(token, &value);
    }

    Ok(RenderedMessageTemplate {
        rendered,
        variables,
    })
}

pub fn extract_variables(body: &str) -> Vec<String> {
    extract_variable_tokens(body)
        .into_iter()
        .map(|(_, variable)| variable)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn extract_variable_tokens(body: &str) -> Vec<(String, String)> {
    let mut variables = BTreeSet::new();
    let mut rest = body;

    while let Some(start) = rest.find("{{") {
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find("}}") else {
            break;
        };
        let variable = after_start[..end].trim();
        if !variable.is_empty()
            && variable
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '.')
        {
            variables.insert((
                format!("{{{{{}}}}}", after_start[..end].to_string()),
                variable.to_string(),
            ));
        }
        rest = &after_start[end + 2..];
    }

    variables.into_iter().collect()
}

fn lookup_context_value(context: &Value, path: &str) -> Option<String> {
    let mut current = context;
    for part in path.split('.') {
        current = current.get(part)?;
    }

    match current {
        Value::Null => Some(String::new()),
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn message_template_renderer_replaces_known_variables() {
        let result = render_template_body(
            "Halo {{ customer.name }}, tagihan {{invoice.number}} sebesar {{invoice.amount}}.",
            &json!({
                "customer": { "name": "Andi" },
                "invoice": { "number": "INV-001", "amount": "Rp100.000" }
            }),
        )
        .expect("template renders");

        assert_eq!(
            result.rendered,
            "Halo Andi, tagihan INV-001 sebesar Rp100.000."
        );
        assert_eq!(
            result.variables,
            vec![
                "customer.name".to_string(),
                "invoice.amount".to_string(),
                "invoice.number".to_string(),
            ]
        );
    }

    #[test]
    fn message_template_renderer_rejects_unknown_variables() {
        let err = render_template_body(
            "Halo {{customer.name}}, {{customer.secret}}",
            &json!({ "customer": { "name": "Andi" } }),
        )
        .unwrap_err();

        assert!(
            matches!(err, AppError::Validation(message) if message.contains("customer.secret"))
        );
    }
}
