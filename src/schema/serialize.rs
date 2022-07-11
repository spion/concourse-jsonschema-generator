use crate::schema::types::*;
use itertools::Itertools;

pub fn prop_type_to_jsonschema_nodocs(prop_type: &PropertyType) -> String {
  prop_type_to_jsonschema(prop_type, None)
}

pub fn prop_type_to_jsonschema(prop_type: &PropertyType, description: Option<&String>) -> String {
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
