//! Custom property commands and the typed property assignment shared by
//! resource commands.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::encode_path_segment;
use crate::cli::{
    CustomPropertyAddArgs, CustomPropertyEditArgs, CustomPropertyGetArgs, CustomPropertyIdArgs,
    CustomPropertyListArgs,
};
use crate::output::Format;
use crate::records::{format_output, format_record, null_to_default, Record};
use crate::runtime::Runtime;
use crate::Outcome;

/// Value types whose values must come from `enumValues`.
const ENUM_VALUE_TYPES: [&str; 2] = ["enum", "multi-enum"];

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CustomProperty {
    id: String,
    key: String,
    label: String,
    resource_type: String,
    value_type: String,
    #[serde(
        default,
        deserialize_with = "null_to_default",
        skip_serializing_if = "Vec::is_empty"
    )]
    enum_values: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    has_associated_data: Option<bool>,
}

impl Record for CustomProperty {
    fn headers() -> &'static [&'static str] {
        &[
            "ID",
            "Key",
            "Label",
            "Resource Type",
            "Value Type",
            "Enum Values",
            "Has Associated Data",
        ]
    }

    fn fields(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.key.clone(),
            self.label.clone(),
            self.resource_type.clone(),
            self.value_type.clone(),
            self.enum_values.join(", "),
            self.has_associated_data
                .map(|value| value.to_string())
                .unwrap_or_default(),
        ]
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CustomPropertyListQuery<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    resource_type: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateCustomPropertyRequest<'a> {
    key: &'a str,
    label: &'a str,
    resource_type: &'a str,
    value_type: &'a str,
    #[serde(skip_serializing_if = "<[String]>::is_empty")]
    enum_values: &'a [String],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EditCustomPropertyRequest<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<&'a str>,
    #[serde(skip_serializing_if = "<[String]>::is_empty")]
    enum_values: &'a [String],
}

#[derive(Deserialize)]
struct CustomPropertyIdResponse {
    id: String,
}

fn custom_property_endpoint(id: &str) -> String {
    format!("/v1/custom-properties/{}", encode_path_segment(id))
}

fn custom_property_not_found(id: &str) -> Outcome {
    Outcome::failure(format!("Custom property not found: {id}\n"))
}

pub(crate) async fn list_custom_properties(
    runtime: &Runtime,
    args: &CustomPropertyListArgs,
    format: Format,
) -> Outcome {
    let query = CustomPropertyListQuery {
        resource_type: args.resource_type.as_deref(),
    };
    match runtime
        .client
        .get::<_, Vec<CustomProperty>>("/v1/custom-properties", &query)
        .await
    {
        Ok(properties) => format_output(&properties, format),
        Err(error) => Outcome::failure(format!("Failed to list custom properties: {error}\n")),
    }
}

pub(crate) async fn get_custom_property(
    runtime: &Runtime,
    args: &CustomPropertyGetArgs,
    format: Format,
) -> Outcome {
    match runtime
        .client
        .get::<_, CustomProperty>(&custom_property_endpoint(&args.custom_property_id), &())
        .await
    {
        Ok(property) => format_record(&property, format),
        Err(error) if error.is_not_found_for("custom property") => {
            custom_property_not_found(&args.custom_property_id)
        }
        Err(error) => Outcome::failure(format!("Failed to get custom property: {error}\n")),
    }
}

pub(crate) async fn add_custom_property(
    runtime: &Runtime,
    args: &CustomPropertyAddArgs,
) -> Outcome {
    let takes_enum_values = ENUM_VALUE_TYPES.contains(&args.value_type.as_str());
    if !takes_enum_values && !args.enum_value.is_empty() {
        return Outcome::failure(format!(
            "--enum-value can only be used with --value-type enum or multi-enum, not {}\n",
            args.value_type
        ));
    }
    if args.value_type == "enum" && args.enum_value.is_empty() {
        return Outcome::failure("--value-type enum requires at least one --enum-value\n");
    }
    let request = CreateCustomPropertyRequest {
        key: &args.key,
        label: &args.label,
        resource_type: &args.resource_type,
        value_type: &args.value_type,
        enum_values: &args.enum_value,
    };
    match runtime
        .client
        .post::<_, CustomPropertyIdResponse>("/v1/custom-properties", &request)
        .await
    {
        Ok(response) => Outcome::notice(format!("Custom property created: {}\n", response.id)),
        Err(error) => Outcome::failure(format!("Failed to create custom property: {error}\n")),
    }
}

pub(crate) async fn edit_custom_property(
    runtime: &Runtime,
    args: &CustomPropertyEditArgs,
) -> Outcome {
    if args.label.is_none() && args.enum_value.is_empty() {
        return Outcome::failure("Nothing to update\n");
    }
    let request = EditCustomPropertyRequest {
        label: args.label.as_deref(),
        enum_values: &args.enum_value,
    };
    match runtime
        .client
        .patch::<_, _, Value>(
            &custom_property_endpoint(&args.custom_property_id),
            &(),
            &request,
        )
        .await
    {
        Ok(_) => Outcome::notice(format!(
            "Custom property updated: {}\n",
            args.custom_property_id
        )),
        Err(error) if error.is_not_found_for("custom property") => {
            custom_property_not_found(&args.custom_property_id)
        }
        Err(error) => Outcome::failure(format!("Failed to edit custom property: {error}\n")),
    }
}

pub(crate) async fn delete_custom_property(
    runtime: &Runtime,
    args: &CustomPropertyIdArgs,
) -> Outcome {
    match runtime
        .client
        .delete(&custom_property_endpoint(&args.custom_property_id))
        .await
    {
        Ok(()) => Outcome::notice(format!(
            "Custom property deleted: {}\n",
            args.custom_property_id
        )),
        Err(error) if error.is_not_found_for("custom property") => {
            custom_property_not_found(&args.custom_property_id)
        }
        Err(error) => Outcome::failure(format!("Failed to delete custom property: {error}\n")),
    }
}

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
    typed_properties(definitions, pairs).map(Some)
}

/// Converts `key:value` pairs to the JSON types their definitions declare.
fn typed_properties(
    definitions: Vec<CustomPropertyDefinition>,
    pairs: &[String],
) -> Result<BTreeMap<String, Value>, String> {
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
    Ok(properties)
}

#[cfg(test)]
mod tests {
    use super::typed_properties;

    fn convert(pairs: &[&str]) -> Result<serde_json::Value, String> {
        let definitions = serde_json::from_value(serde_json::json!([
            {"key": "note", "valueType": "string"},
            {"key": "speed", "valueType": "number"},
            {"key": "accepted", "valueType": "boolean"},
            {"key": "weather", "valueType": "enum", "enumValues": ["rain", "sun"]},
            {"key": "tags", "valueType": "multi-enum", "enumValues": ["outdoor", "night"]},
            {"key": "location", "valueType": "geo"},
        ]))
        .unwrap();
        let pairs: Vec<String> = pairs.iter().map(|&pair| pair.to_owned()).collect();
        typed_properties(definitions, &pairs).map(|properties| serde_json::json!(properties))
    }

    #[test]
    fn values_take_the_type_of_their_definition() {
        assert_eq!(
            convert(&[
                "note:night: run",
                "speed:3.5",
                "accepted:F",
                "weather:rain",
                "tags:night",
                "tags:outdoor",
                "tags:night",
            ]),
            Ok(serde_json::json!({
                "note": "night: run",
                "speed": 3.5,
                "accepted": false,
                "weather": "rain",
                "tags": ["night", "outdoor"],
            }))
        );
    }

    #[test]
    fn values_that_do_not_fit_their_definition_are_rejected() {
        for (pair, error) in [
            ("weather", "invalid key/value pair: weather"),
            (":rain", "invalid key/value pair: :rain"),
            ("unknown:value", "unknown key: unknown"),
            ("speed:NaN", "invalid value for number: NaN"),
            ("accepted:maybe", "invalid value for boolean: maybe"),
            ("weather:snow", "invalid enum value: snow"),
            ("tags:day", "invalid enum value: day"),
            ("location:here", "unsupported type: geo"),
        ] {
            assert_eq!(convert(&[pair]), Err(error.to_owned()), "{pair}");
        }
    }
}
