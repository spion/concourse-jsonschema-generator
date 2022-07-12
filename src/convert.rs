use crate::lit::types::{LitDocument, LitNode};
use crate::schema::types::{Property, PropertyType, Schema};

pub fn transform_to_jsonschemas(doc: &LitDocument) -> Vec<Schema> {
  doc
    .iter()
    .flat_map(|node| {
      let mut group_members: Vec<String> = vec![];

      match node {
        LitNode::Text(_) => vec![],
        // TODO: collect text here
        LitNode::Fn(schema, args) if (schema == "schema") || (schema == "schema-group") => {
          let mut found_schemas: Vec<Schema> = vec![];

          let schema_name = text_to_markdown(&args[0])
            .trim()
            .replace("`", "_")
            .replace("-", "_")
            .replace(" ", "_")
            .replace("__", "_")
            .trim_start_matches("_")
            .to_string();

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
              let type_name = text_to_markdown(&args[1]).trim().to_string();

              let is_list = type_name.starts_with("[");

              let documentation = &args[2];

              let inner_schemas = transform_to_jsonschemas(documentation);
              found_schemas.extend(inner_schemas);

              let prop_name = text_to_markdown(&args[0]).trim().to_string();

              vec![(
                prop_name,
                Property {
                  required: prop == "required-attribute",
                  docs: text_to_markdown(documentation).trim().to_string(),
                  type_name: parse_type(&type_name.replace("-", "_")),
                  list: is_list,
                },
              )]
            }
            LitNode::Fn(_other_fn, args) => {
              let inner_schemas = args
                .into_iter()
                .flat_map(transform_to_jsonschemas)
                .collect::<Vec<_>>();

              group_members.extend(
                inner_schemas
                  .iter()
                  .filter(|s| s.part_of_group)
                  .map(|s| s.schema_name.clone())
                  .collect::<Vec<_>>(),
              );

              found_schemas.extend(inner_schemas);

              vec![]
            }
            _ => vec![], //panic!("Unexpected non-property function call in schema"),
          })
          .collect();

          found_schemas.push(Schema {
            part_of_group: schema == "schema-group",
            group_members: group_members,
            schema_name: schema_name, //snake_to_pascal(

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

peg::parser! {
  grammar lit_type_parser() for str {

    pub rule lit_type() -> PropertyType
      = union_type() / non_union_type()

    rule non_union_type() -> PropertyType
      = array_type() / dictionary_type() / constant_type() / ref_type()

    rule array_type() -> PropertyType
      = "[" inner_type:lit_type() "]" { PropertyType::ArrayOf(Box::new(inner_type)) }

    rule union_type() -> PropertyType =
      inner_types:(non_union_type() ++ (_ "|" _)) { PropertyType::OneOf(inner_types) }

    rule _ = [' ' | '\n']*;

    rule key_or_value_string() -> String
      = name:$(['a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_']+) { String::from(name) }

    rule type_identifier() -> String
      = name:$(['a'..='z' | 'A'..='Z' | '_']+) { String::from(name) }

    rule dictionary_type() -> PropertyType
      = "{" _ key_or_value_string() _ ":" _ key_or_value_string() "}" { PropertyType::Dict }

    rule constant_type() -> PropertyType
      = "`" value:key_or_value_string() "`" { PropertyType::Constant(value) }

    rule ref_type() -> PropertyType
      = name:key_or_value_string() {
        PropertyType::Ref(
          if name.contains(".") { "string".to_string() } else { name }
        )
      }


  }
}

fn parse_type(s: &str) -> PropertyType {
  match lit_type_parser::lit_type(s) {
    Ok(res) => res,
    Err(e) => {
      println!("Error parsing type: {}", s);
      println!("{}", e);
      panic!("Unable to parse type")
    }
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
      LitNode::Fn(_any_, args) => args.iter().map(text_to_markdown).collect(),
      _ => "".to_string(),
    })
    .collect::<String>()
    .to_string()
    .replace("\\{", "{")
    .replace("\\}", "}")
}

pub fn clean_text(text: &str) -> String {
  text
    .lines()
    // TODO: Do not trim beginning of first and end of last
    .map(|t| " ".to_string() + t.trim() + " ")
    .map(|t| if t == "" { "\n\n".to_string() } else { t })
    .collect()
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
