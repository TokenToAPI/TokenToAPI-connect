use crate::{
    models::ValidatedInput,
    provider_fields,
    storage::{self, Change, FileKind, Paths},
};
use jsonc_parser::{
    cst::{CstInputValue, CstRootNode},
    ParseOptions,
};
use serde_json::{json, Value};

fn cst(value: Value) -> CstInputValue {
    match value {
        Value::Null => CstInputValue::Null,
        Value::Bool(v) => CstInputValue::Bool(v),
        Value::Number(v) => CstInputValue::Number(v.to_string()),
        Value::String(v) => CstInputValue::String(v),
        Value::Array(v) => CstInputValue::Array(v.into_iter().map(cst).collect()),
        Value::Object(v) => {
            CstInputValue::Object(v.into_iter().map(|(k, v)| (k, cst(v))).collect())
        }
    }
}

fn options() -> ParseOptions {
    ParseOptions {
        allow_comments: true,
        allow_trailing_commas: true,
        allow_loose_object_property_names: false,
        allow_missing_commas: false,
        allow_single_quoted_strings: false,
        allow_hexadecimal_numbers: false,
        allow_unary_plus_numbers: false,
        allow_bare_decimal_point_numbers: false,
        allow_non_finite_numbers: false,
        allow_extended_string_escapes: false,
    }
}

pub fn read(paths: &Paths) -> Result<Value, String> {
    let text = storage::read_optional(&paths.target(FileKind::ClaudeVscode))?;
    let text = text
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("{}");
    let doc: Value =
        jsonc_parser::parse_to_serde_value(text.trim_start_matches('\u{feff}'), &options())
            .map_err(|_| {
                "The VS Code user settings are not valid JSONC; the configuration was not modified."
            })?;
    if !doc.is_object() {
        return Err(
            "The VS Code user settings must be an object; the configuration was not modified."
                .into(),
        );
    }
    Ok(doc)
}

pub fn change(paths: &Paths, input: &ValidatedInput) -> Result<Change, String> {
    let kind = FileKind::ClaudeVscode;
    let before = storage::read_optional(&paths.target(kind))?;
    let text = before
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("{}");
    let options = options();
    let key = "claudeCode.environmentVariables";
    let doc: Value = jsonc_parser::parse_to_serde_value(text, &options).map_err(|_| {
        "The VS Code user settings are not valid JSONC; the configuration was not modified."
    })?;
    if !doc.is_object() {
        return Err(
            "The VS Code user settings must be an object; the configuration was not modified."
                .into(),
        );
    }
    let mut vars = match doc.get(key) {
        None => vec![],
        Some(Value::Array(values)) => values.clone(),
        _ => return Err(
            "claudeCode.environmentVariables must be an array; the configuration was not modified."
                .into(),
        ),
    };
    if vars.iter().any(|v| {
        v.get("name").and_then(Value::as_str).is_none()
            || v.get("value").and_then(Value::as_str).is_none()
    }) {
        return Err("The Claude VS Code environment variables have an invalid format; the configuration was not modified.".into());
    }
    vars.retain(|v| {
        let name = v["name"].as_str().unwrap();
        if input.options.artifact_mode == crate::setup_options::ArtifactMode::Preserve {
            !provider_fields::claude_floor_env_preserving_artifact(name)
        } else {
            !provider_fields::claude_floor_env(name)
        }
    });
    for (name, value) in crate::claude_config::environment(input) {
        vars.retain(|v| v["name"] != name);
        vars.push(json!({"name": name, "value": value}));
    }
    // CST replaces only the extension's property; all surrounding comments and
    // settings stay byte-for-byte intact, including trailing commas and URLs.
    let root =
        CstRootNode::parse(text, &options).map_err(|_| "Could not parse the VS Code settings.")?;
    let obj = root.object_value_or_set();
    if obj
        .properties()
        .iter()
        .filter(|prop| prop.decoded_name().as_deref() == Some(key))
        .count()
        > 1
    {
        return Err(
            "VS Code contains duplicate claudeCode.environmentVariables settings; merge them before importing.".into(),
        );
    }
    if let Some(property) = obj.get(key) {
        property.set_value(cst(json!(vars)));
    } else {
        obj.append(key, cst(json!(vars)));
    }
    if obj
        .properties()
        .iter()
        .filter(|prop| prop.decoded_name().as_deref() == Some("claudeCode.disableLoginPrompt"))
        .count()
        > 1
    {
        return Err("VS Code contains duplicate claudeCode.disableLoginPrompt settings; merge them before importing.".into());
    }
    if let Some(property) = obj.get("claudeCode.disableLoginPrompt") {
        property.set_value(CstInputValue::Bool(true));
    } else {
        obj.append("claudeCode.disableLoginPrompt", CstInputValue::Bool(true));
    }
    Ok(Change {
        kind,
        before,
        after: root.to_string(),
    })
}
