use serde_json::{json, Value};

pub(crate) const FIELDS: [&str; 4] = [
    "context_window",
    "max_context_window",
    "auto_compact_token_limit",
    "effective_context_window_percent",
];

fn limits(row: &Value) -> Value {
    let mut result = json!({});
    for key in FIELDS {
        result[key] = row.get(key).cloned().unwrap_or_else(|| {
            if key == "effective_context_window_percent" {
                json!(95)
            } else {
                Value::Null
            }
        });
    }
    result
}

pub(crate) fn validate(row: &Value, required: bool) -> Result<(), String> {
    for key in FIELDS {
        let value = &row[key];
        if value.is_null() && key != "effective_context_window_percent" {
            continue;
        }
        let upper = if key == "effective_context_window_percent" {
            100
        } else {
            i64::MAX / 100
        };
        if !value
            .as_i64()
            .is_some_and(|number| number > 0 && number <= upper)
        {
            return Err("The model context capacity or compaction setting is invalid; no configuration was written.".into());
        }
    }
    if let (Some(window), Some(maximum)) = (
        row["context_window"].as_i64(),
        row["max_context_window"].as_i64(),
    ) {
        if window > maximum {
            return Err(
                "The model context window exceeds its maximum capacity; no configuration was written."
                    .into(),
            );
        }
    }
    if required
        && row["context_window"]
            .as_i64()
            .or(row["max_context_window"].as_i64())
            .is_none()
    {
        return Err(format!(
            "Model {} has no declared context capacity. Ask the gateway to provide context_window or max_context_window, or deselect this model; no client configuration was changed.",
            row["slug"].as_str().unwrap_or("unknown")
        ));
    }
    Ok(())
}

fn find_model<'a>(catalog: &'a Value, id: &str) -> Option<&'a Value> {
    catalog.get("models")?.as_array()?.iter().find(|row| {
        row.get("slug").and_then(Value::as_str) == Some(id)
            || row.get("id").and_then(Value::as_str) == Some(id)
    })
}

fn legacy_default(row: &Value) -> bool {
    row["context_window"] == 32768
        && row["max_context_window"] == 32768
        && row["auto_compact_token_limit"].is_null()
        && row["effective_context_window_percent"] == 95
}

pub(crate) fn prepare_import(
    generated: &str,
    previous: Option<&str>,
    cached: Option<&str>,
) -> Result<String, String> {
    let mut catalog: Value = serde_json::from_str(generated)
        .map_err(|_| "The generated model catalog is invalid; no configuration was written.")?;
    let previous: Option<Value> = previous
        .map(serde_json::from_str)
        .transpose()
        .map_err(|_| "The current model catalog is invalid; no configuration was written.")?;
    let cached: Option<Value> = cached.and_then(|text| serde_json::from_str(text).ok());
    let gateway_fields = catalog["_tokentoapi_connect"]["gateway_context_fields"].clone();
    let mut modality_sources = catalog["_tokentoapi_connect"]["input_modality_sources"].clone();
    let mut generated_context = json!({});
    let rows = catalog["models"]
        .as_array_mut()
        .ok_or("The generated model catalog has no models; no configuration was written.")?;

    for row in rows {
        let id = row["slug"]
            .as_str()
            .ok_or("The generated model catalog has an invalid model name.")?
            .to_string();
        let gateway_owns = |key: &str| {
            gateway_fields[&id]
                .as_array()
                .is_some_and(|fields| fields.iter().any(|field| field == key))
        };

        // Native metadata can cover newer models without a Connect release.
        // Never replace an explicit gateway limit or a verified bundled profile.
        if modality_sources[&id] == "unknown" {
            if let Some(items) = cached
                .as_ref()
                .and_then(|value| find_model(value, &id))
                .and_then(|model| model["input_modalities"].as_array())
            {
                let modalities: Vec<_> = ["text", "image"]
                    .into_iter()
                    .filter(|modality| items.iter().any(|item| item == modality))
                    .collect();
                if !modalities.is_empty() {
                    row["input_modalities"] = json!(modalities);
                    modality_sources[&id] = json!("native-cache");
                }
            }
        }

        if row["context_window"].is_null()
            && row["max_context_window"].is_null()
            && !gateway_owns("context_window")
            && !gateway_owns("max_context_window")
        {
            if let Some(cached_row) = cached.as_ref().and_then(|value| find_model(value, &id)) {
                let cached_limits = limits(cached_row);
                if validate(&cached_limits, true).is_ok() {
                    for key in FIELDS {
                        if !gateway_owns(key) {
                            row[key] = cached_limits[key].clone();
                        }
                    }
                }
            }
        }

        if row["context_window"].is_null() && !row["max_context_window"].is_null() {
            row["context_window"] = row["max_context_window"].clone();
        } else if row["max_context_window"].is_null() && !row["context_window"].is_null() {
            row["max_context_window"] = row["context_window"].clone();
        }
        let generated = limits(row);
        generated_context[&id] = generated.clone();
        if let Some(previous_catalog) = previous.as_ref() {
            if let Some(old) = find_model(previous_catalog, &id) {
                let old_limits = limits(old);
                let recorded = &previous_catalog["_tokentoapi_connect"]["generated_context"][&id];
                for key in FIELDS {
                    let manually_changed = if recorded.is_object() {
                        old_limits[key] != recorded[key]
                    } else {
                        !legacy_default(&old_limits) && old_limits[key] != generated[key]
                    };
                    if manually_changed && !gateway_owns(key) {
                        row[key] = old_limits[key].clone();
                    }
                }
            }
        }
        if row["context_window"].is_null() && !row["max_context_window"].is_null() {
            row["context_window"] = row["max_context_window"].clone();
        } else if row["max_context_window"].is_null() && !row["context_window"].is_null() {
            row["max_context_window"] = row["context_window"].clone();
        }
        validate(row, true)?;
    }

    catalog["_tokentoapi_connect"]["schema_version"] = json!(1);
    catalog["_tokentoapi_connect"]["generated_context"] = generated_context;
    catalog["_tokentoapi_connect"]["input_modality_sources"] = modality_sources;
    serde_json::to_string_pretty(&catalog)
        .map_err(|_| "Could not generate the model catalog.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_32k_default_is_not_treated_as_a_user_repair() {
        let old = json!({"models":[{"slug":"model","context_window":32768,
            "max_context_window":32768,"effective_context_window_percent":95}]});
        assert!(legacy_default(&old["models"][0]));
    }
}
