//! Typed custom property assignment shared by resource commands.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

use crate::runtime::Runtime;

#[derive(Deserialize)]
struct CustomPropertyDefinition {
    key: String,
    #[serde(rename = "valueType")]
    value_type: String,
    #[serde(rename = "enumValues", default)]
    enum_values: Vec<String>,
}

pub(crate) async fn properties(
    runtime: &Runtime,
    resource_type: &str,
    pairs: &[String],
) -> Result<Option<BTreeMap<String, Value>>, String> {
    if pairs.is_empty() {
        return Ok(None);
    }
    let query = [("resourceType", resource_type)];
    let definitions = runtime
        .client
        .get::<_, Vec<CustomPropertyDefinition>>("/v1/custom-properties", &query)
        .await
        .map_err(|error| format!("failed to load custom properties: {error}"))?;
    let definitions = definitions
        .into_iter()
        .map(|definition| (definition.key.clone(), definition))
        .collect::<BTreeMap<_, _>>();
    let mut properties = BTreeMap::new();
    for pair in pairs {
        let Some((key, raw_value)) = pair.split_once(':') else {
            return Err(format!("invalid key/value pair: {pair}"));
        };
        if key.is_empty() {
            return Err(format!("invalid key/value pair: {pair}"));
        }
        let Some(definition) = definitions.get(key) else {
            return Err(format!("unknown key: {key}"));
        };
        let value = match definition.value_type.as_str() {
            "string" | "multiline-string" => Value::String(raw_value.to_owned()),
            "number" => {
                let number = raw_value
                    .parse::<f64>()
                    .map_err(|_| format!("invalid value for number: {raw_value}"))?;
                serde_json::from_str(&number.to_string())
                    .map_err(|_| format!("invalid value for number: {raw_value}"))?
            }
            "boolean" => match raw_value {
                "1" | "t" | "T" | "TRUE" | "true" | "True" => Value::Bool(true),
                "0" | "f" | "F" | "FALSE" | "false" | "False" => Value::Bool(false),
                _ => return Err(format!("invalid value for boolean: {raw_value}")),
            },
            "enum"
                if definition
                    .enum_values
                    .iter()
                    .any(|value| value == raw_value) =>
            {
                Value::String(raw_value.to_owned())
            }
            "multi-enum"
                if definition
                    .enum_values
                    .iter()
                    .any(|value| value == raw_value) =>
            {
                let mut values = match properties.remove(key) {
                    Some(Value::Array(values)) => values,
                    _ => Vec::new(),
                };
                let value = Value::String(raw_value.to_owned());
                if !values.contains(&value) {
                    values.push(value);
                }
                Value::Array(values)
            }
            "enum" | "multi-enum" => return Err(format!("invalid enum value: {raw_value}")),
            other => return Err(format!("unsupported type: {other}")),
        };
        properties.insert(key.to_owned(), value);
    }
    Ok(Some(properties))
}
