use std::collections::HashMap;

use crate::schema::types::*;
use itertools::Itertools;
use serde_json::{json, Value};

fn merge(a: &mut Value, b: &Value) {
  match (a, b) {
    (&mut Value::Object(ref mut a), &Value::Object(ref b)) => {
      for (k, v) in b {
        merge(a.entry(k.clone()).or_insert(Value::Null), v);
      }
    }
    (a, b) => {
      *a = b.clone();
    }
  }
}

pub fn serialize(schema_docs: &HashMap<String, Schema>) -> String {
  let definitions = schema_docs
    .iter()
    .map(|(schema_name, schema)| {
      let schema_props = schema
        .properties
        .iter()
        .map(|(prop_name, prop)| {
          (
            prop_name,
            prop_type_to_jsonschema(&prop.type_name, Some(&prop.docs)),
          )
        })
        .collect::<HashMap<_, _>>();

      let subschemas = if schema.group_members.len() > 0 {
        prop_type_to_jsonschema(
          &PropertyType::OneOf(
            schema
              .group_members
              .iter()
              .map(|m| PropertyType::Ref(m.to_string()))
              .collect(),
          ),
          None,
        )
      } else {
        json!({})
      };

      let required_props = schema
        .properties
        .iter()
        .filter(|(_, prop)| prop.required)
        .map(|(name, _)| name)
        .collect_vec();

      let mut result = json!({});

      // let additional_properties =
      //   schema.is_group_member || (schema.group_members.len() > 0 && schema.properties.len() > 0);

      if schema_props.len() > 1 {
        merge(
          &mut result,
          &json!({
            "additionalProperties":false,
            "required":required_props,
            "type":"object",
            "properties":schema_props
          }),
        );
      }

      if schema.group_members.len() > 0 {
        merge(&mut result, &subschemas)
      }

      log::debug!(
        "Schema {} has {} props and {} group members",
        schema_name,
        schema_props.len(),
        schema.group_members.len()
      );

      let schema_obj = if schema.group_members.len() > 0 || schema.properties.len() > 0 {
        result
      } else if schema_name == "number" {
        json!({"type":"number"})
      } else if schema_name == "boolean" {
        json!({"type":"boolean"})
      } else if schema_name == "value" {
        json!({})
      } else if schema_name == "config" || schema_name == "vars" {
        json!({"type":"object","patternProperties":{".*":{"additionalProperties":true}}})
      } else if schema_name == "env_vars" || schema_name == "version" {
        json!({"type":"object","patternProperties":{".*":{"type":"string"}}})
      } else {
        json!({"type": "string"})
      };

      (schema_name, schema_obj)
    })
    .collect::<HashMap<_, _>>();

  json!({
    "$schema": "http://json-schema.org/draft-04/schema#",
    "$ref": "#/definitions/pipeline",
    "additionalProperties": true,
    "definitions": definitions
  })
  .to_string()
}

fn prop_type_to_jsonschema_nodocs(prop_type: &PropertyType) -> Value {
  prop_type_to_jsonschema(prop_type, None)
}

fn prop_type_to_jsonschema(prop_type: &PropertyType, description: Option<&String>) -> Value {
  let mut prop_schema = match prop_type {
    PropertyType::OneOf(types) => {
      let all_types_count = types.len();

      assert!(all_types_count > 0);

      let all_strings = types
        .iter()
        .map(|p| match p {
          PropertyType::Constant(c) => c.to_string(),
          _ => "".to_string(),
        })
        .filter(|p| p != "")
        .collect::<Vec<_>>();

      if all_strings.len() == all_types_count {
        log::debug!("Enum type {:?}", prop_type);
        json!({"type":"string","enum":all_strings})
      } else if all_types_count == 1 {
        prop_type_to_jsonschema_nodocs(&types[0])
      } else {
        json!({"oneOf": types.iter().map(prop_type_to_jsonschema_nodocs).collect_vec()})
      }
    }
    PropertyType::ArrayOf(inner) => json!({
      "type":"array",
      "items":prop_type_to_jsonschema(inner, description)
    }),
    PropertyType::Constant(item) => json!({
      "type":"string",
      "enum":[item]
    }),
    PropertyType::Dict => {
      json!({
        "type":"object",
        "patternProperties":{".*":{"type":"string"}}
      })
    }
    PropertyType::Ref(item) => {
      json!({
        "$ref":"#/definitions/".to_string() + item.replace("\\", "\\\\").as_str()
      })
    }
  };

  match description {
    Some(docs) => merge(&mut prop_schema, &json!({ "description": docs })),
    None => (),
  };

  prop_schema
}
