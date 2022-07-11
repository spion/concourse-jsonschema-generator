use crate::lit::types::{LitDocument, LitNode};
use crate::schema::types::{Property, PropertyType, Schema};

pub fn transform_to_jsonschemas(doc: &LitDocument) -> Vec<Schema> {
  doc
    .iter()
    .flat_map(|node| {
      match node {
        LitNode::Text(_) => vec![],
        // TODO: collect text here
        LitNode::Fn(schema, args) if (schema == "schema") || (schema == "schema-group") => {
          let mut found_schemas: Vec<Schema> = vec![];

          let props = (if schema == "schema" {
            &args[1]
          } else {
            &args[2]
          })
          .iter()
          .flat_map(|node| match node {
            LitNode::Text(_) => {
              vec![]
            }
            LitNode::Fn(prop, args)
              if (prop == "required-attribute" || prop == "optional-attribute") =>
            {
              let type_name = raw_text(&args[1]).trim().to_string();

              let is_list = type_name.starts_with("[");

              let documentation = &args[2];

              found_schemas.extend(transform_to_jsonschemas(documentation));

              let prop_name = text_to_markdown(&args[0]);
              vec![(
                prop_name,
                Property {
                  required: prop == "required-attribute",
                  docs: text_to_markdown(documentation),
                  type_name: parse_type(&type_name.replace("-", "_")),
                  list: is_list,
                },
              )]
            }
            LitNode::Fn(_other_fn, args) => {
              found_schemas.extend(
                args
                  .into_iter()
                  .flat_map(transform_to_jsonschemas)
                  .collect::<Vec<_>>(),
              );

              vec![]
            }
            _ => vec![], //panic!("Unexpected non-property function call in schema"),
          })
          .collect();

          found_schemas.push(Schema {
            schema_name: //snake_to_pascal(
              text_to_markdown(&args[0])
                .replace("`", "_")
                .replace("-", "_")
                .replace(" ", "_")
                .replace("__", "_")
                .trim_start_matches("_").to_string(),
            //),
            properties: props,
          });

          found_schemas.extend(args.into_iter().flat_map(transform_to_jsonschemas));

          return found_schemas;
        }
        LitNode::Fn(_other_fn, args) => args
          .into_iter()
          .flat_map(transform_to_jsonschemas)
          .collect(),

        LitNode::Comment(_) => vec![],
      }
    })
    .collect()
}

fn parse_type(s: &str) -> PropertyType {
  if s.starts_with("[") && s.ends_with("]") {
    return PropertyType::ArrayOf(Box::new(parse_type(
      s.trim_start_matches("[").trim_end_matches("]"),
    )));
  }
  let multi = s.split("|").count();
  if multi > 1 {
    PropertyType::OneOf(s.split("|").map(|t| parse_type(t.trim())).collect())
  } else if s.starts_with("`") && s.ends_with("`") {
    PropertyType::Constant(s.trim_start_matches("`").trim_end_matches("`").to_string())
  } else {
    PropertyType::Ref(s.to_string())
  }
}

pub fn text_to_markdown(nodes: &Vec<LitNode>) -> String {
  nodes
    .iter()
    .map(|n| match n {
      LitNode::Text(t) => clean_text(t),
      LitNode::Fn(example_fn, args) if (example_fn == "example-toggle") => {
        format!(
          "@example {}\n{}",
          text_to_markdown(&args[0]),
          text_to_markdown(&args[1])
        )
      }
      LitNode::Fn(codeblock, args) if (codeblock == "codeblock") => {
        format!(
          "```{}\n{}\n```",
          raw_text(&args[0]).trim(),
          trim_codeblock(&raw_text(&args[1]))
        )
      }
      LitNode::Fn(code, args) if (code == "code") => {
        format!("`{}`", raw_text(&args[0]))
      }
      LitNode::Fn(bold, args) if (bold == "bold") => {
        format!("**{}**", text_to_markdown(&args[0]))
      }
      LitNode::Fn(warn, args) if (warn == "warn") => text_to_markdown(&args[0]),
      _ => "".to_string(),
    })
    .collect::<String>()
    .trim()
    .to_string()
    .replace("\\{", "{")
    .replace("\\}", "}")
}

pub fn clean_text(text: &str) -> String {
  text
    .lines()
    .map(|t| t.trim())
    .collect::<Vec<_>>()
    .join("\n")
}

pub fn trim_codeblock(text: &str) -> String {
  // println!("Codeblock\n{}\n", text);

  let trim_start_count = text
    .lines()
    .filter(|l| l.len() > 0)
    .map(|s| s.chars().position(|c| c != ' '))
    .filter(|x| x.is_some())
    .map(|x| x.unwrap())
    .min()
    .unwrap_or(0);

  // println!("Trim Start = {}", trim_start_count);

  text
    .split("\n")
    .map(|l| {
      if l.len() > trim_start_count {
        &l[trim_start_count..]
      } else {
        l.trim()
      }
    })
    .collect::<Vec<_>>()
    .join("\n")
    .trim()
    .to_string()
}

pub fn raw_text(nodes: &Vec<LitNode>) -> String {
  nodes
    .iter()
    .map(|n| match n {
      LitNode::Text(t) => t.as_str(),
      _ => "",
    })
    .collect::<String>()
}