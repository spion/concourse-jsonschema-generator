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

      assert!(all_types_count > 0);

      let all_strings = types
        .iter()
        .map(|p| match p {
          PropertyType::Constant(c) => format!("\"{}\"", c),
          _ => "".to_string(),
        })
        .filter(|p| p != "")
        .collect::<Vec<_>>();

      if all_strings.len() == all_types_count {
        log::debug!("Enum type {:?}", prop_type);

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
        let oneof_def = if schema.group_members.len() > 0 {
          let one_of = prop_type_to_jsonschema(
            &PropertyType::OneOf(
              schema
                .group_members
                .iter()
                .map(|m| PropertyType::Ref(m.to_string()))
                .collect(),
            ),
            None,
          );

          one_of[1..one_of.len() - 1].to_owned() + ","
        } else {
          "".to_owned()
        };

        let required_props = schema
          .properties
          .iter()
          .filter(|(_, prop)| prop.required)
          .map(|(name, _)| format!("\"{}\"", name))
          .join(",");

        let props_object = format!(
          r#""additionalProperties":{},"required":[{}],"type":"object","properties":{{{}}}"#,
          if schema.is_group_member
            || (schema.group_members.len() > 0 && schema.properties.len() > 0)
          {
            true
          } else {
            false
          },
          required_props,
          schema_props.join(",")
        );

        log::debug!(
          "Schema {} has {} props and {} group members",
          schema_name,
          schema_props.len(),
          schema.group_members.len()
        );
        // if schema.group_members.len() > 0 && schema_props.len() > 0 {
        format!(r#"{{{}{}}}"#, oneof_def, props_object)
        // } else if schema_props.len() > 0 {
        // assert!(props_object.len() > 0);
        // props_object
        // } else {
        // assert!(oneof_def.len() > 0);
        // oneof_def
        // }
      } else if schema_name == "number" {
        r#"{"type":"number"}"#.to_string()
      } else if schema_name == "boolean" {
        r#"{"type":"boolean"}"#.to_string()
      } else if schema_name == "value" {
        "{}".to_string()
      } else if schema_name == "config" || schema_name == "vars" {
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
