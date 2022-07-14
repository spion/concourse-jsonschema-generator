use std::collections::HashMap;

use crate::schema::types::*;
use itertools::Itertools;

fn prop_type_to_jsonschema_nodocs(prop_type: &PropertyType) -> String {
  prop_type_to_jsonschema(prop_type, None)
}

fn prop_type_to_jsonschema(prop_type: &PropertyType, description: Option<&String>) -> String {
  let additional = match prop_type {
    PropertyType::OneOf(types) => {
      let all_types_count = types.len();

      let all_strings = types
        .iter()
        .map(|p| match p {
          PropertyType::Constant(c) => format!("\"{}\"", c),
          _ => "".to_string(),
        })
        .filter(|p| p != "")
        .collect::<Vec<_>>();

      if all_strings.len() == all_types_count {
        format!(r#""type":"string","enum":[{}]"#, all_strings.join(","))
      } else if all_types_count == 1 {
        let inner = prop_type_to_jsonschema_nodocs(&types[0]);
        inner[1..inner.len() - 1].to_string()
      } else {
        format!(
          r#""oneOf":[{}]"#,
          types.iter().map(prop_type_to_jsonschema_nodocs).join(",")
        )
      }
    }
    PropertyType::ArrayOf(inner) => format!(
      r#""type":"array","items":{}"#,
      prop_type_to_jsonschema(inner, description)
    ),
    PropertyType::Constant(item) => format!(r#""type":"string","enum":["{}"]"#, item),
    PropertyType::Dict => {
      r#""type":"object","patternProperties":{".*":{"type":"string"}}"#.to_string()
    }
    PropertyType::Ref(item) => {
      format!(r##""$ref":"#/definitions/{}""##, item.replace("\\", "\\\\"))
    }
  };

  let desc = match description {
    Some(docs) => format!(
      r#""description":"{}","#,
      docs.replace("\"", "\\\"").replace("\n", "\\n")
    ),
    None => "".to_string(),
  };

  format!(r#"{{{}{}}}"#, desc, additional)
}

pub fn serialize(schema_docs: &HashMap<String, Schema>) -> String {
  let definitions = schema_docs
    .iter()
    .map(|(schema_name, schema)| {
      let schema_props = schema
        .properties
        .iter()
        .map(|(prop_name, prop)| {
          format!(
            r#""{}":{}"#,
            prop_name,
            prop_type_to_jsonschema(&prop.type_name, Some(&prop.docs))
          )
        })
        .collect::<Vec<_>>();

      let schema_obj = if schema_props.len() > 0 || schema.group_members.len() > 0 {
        log::debug!(
          "Schema {} has {} props and {} group members",
          schema_name,
          schema_props.len(),
          schema.group_members.len()
        );
        let oneof_def = prop_type_to_jsonschema(
          &PropertyType::OneOf(
            schema
              .group_members
              .iter()
              .map(|m| PropertyType::Ref(m.to_string()))
              .collect(),
          ),
          None,
        );

        let required_props = schema
          .properties
          .iter()
          .filter(|(_, prop)| prop.required)
          .map(|(name, _)| format!("\"{}\"", name))
          .join(",");

        let props_object = format!(
          r#"{{"additionalProperties":false,"required":[{}],"type":"object","properties":{{{}}}}}"#,
          required_props,
          schema_props.join(",")
        );

        if schema.group_members.len() > 0 && schema_props.len() > 0 {
          format!(r#"{{"allOf":[{},{}]}}"#, props_object, oneof_def)
        } else if schema_props.len() > 1 {
          props_object
        } else {
          oneof_def
        }
      } else if schema_name == "number" {
        r#"{"type":"number"}"#.to_string()
      } else if schema_name == "boolean" {
        r#"{"type":"boolean"}"#.to_string()
      } else if schema_name == "config" || schema_name == "value" || schema_name == "vars" {
        r#"{"type":"object","patternProperties":{".*":{"additionalProperties":true}}}"#.to_string()
      } else if schema_name == "env_vars" || schema_name == "version" {
        r#"{"type":"object","patternProperties":{".*":{"type":"string"}}}"#.to_string()
      } else {
        r#"{"type":"string"}"#.to_string()
      };

      format!(r#""{}":{}"#, schema_name, schema_obj)
    })
    .join(",");

  format!(
    r###"
  {{
    "$schema": "http://json-schema.org/draft-04/schema#",
    "$ref": "#/definitions/pipeline",
    "additionalProperties": true,
    "definitions":{{{}}}
  }}
  "###,
    definitions
  )
}
