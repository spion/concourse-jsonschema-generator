use peg::error::ParseError;
use peg::str::LineCol;
use std::collections::HashMap;
use std::fs;
// use itertools::{join};

use serde::{Serialize, Deserialize};
use clap::Parser;

type Document = Vec<LitNode>;

#[derive(Debug)]
pub enum LitNode {
  Text(String),
  Fn(String, Vec<Document>),
  Comment(String),
}

#[derive(Debug)]
pub struct Schema {
  name: String,
  properties: HashMap<String, Property>,
}

#[derive(Debug)]
pub struct Property {
  type_name: String,
  required: bool,
  list: bool,
  docs: String,
}

#[derive(Debug)]
pub enum Json {
  String(String),
  Object(HashMap<String, Json>),
  Array(Vec<Json>),
  Bool(bool),
  Number(String),
  Null,
}


peg::parser! {
  grammar json_parser() for str {


    pub rule json() -> Json
      = _ val:(string_val() / object() / number() / null() / bool() / array()) _
      { val }

    rule _
      = [' ' | '\n']*

    rule string() -> String
      = _ "\"" content:$(("\\\"" / [^ '"']+)+) "\"" _ { content.to_string() }

    rule string_val() -> Json
      = content:string() { Json::String(content) }

    rule kvpair() -> (String, Json)
      = key:(string()) ":" value:(json()) { (key, value) }

    rule object() -> Json
      = "{" kvpairs:(kvpair() ** ",") "}" {
        Json::Object(kvpairs.into_iter().collect())
      }

    rule number() -> Json
      = num:$("-"? ['.' | '0'..='9']+) { Json::Number(num.to_string()) }

    rule bool() -> Json
      = b:$("true" / "false") { Json::Bool(b == "true") }

    rule array() -> Json
      = "[" l:(json() ** ",") "]" { Json::Array(l) }

    rule null() -> Json
      = "null" { Json::Null }
  }
}

peg::parser! {
  grammar lit_parser() for str {
    pub rule doc() -> Document
      = l:(LitNode() *) { l }

    rule LitNode() -> LitNode
      = functionCall() / comment() / textContent()

    rule comment() -> LitNode
      = "{-" content:(anyComment()) "-}" { LitNode::Comment(content) }

    rule anyComment() -> String
      = content:$((!"-}" [_])+) {
        String::from(content)
      }

    rule functionCall() -> LitNode
      = "\\" fnName:(functionName()) args:(argument()*) {
        LitNode::Fn(fnName, args)
      }

    rule functionName() -> String
      = name:$([ 'a'..='z' | 'A'..='Z']['a'..='z' | 'A'..='Z' | '0'..='9' | '-' ]+
      / expected!("function-name")) { String::from(name) }

    rule argument() -> Document
      = verbatimArgument() / preformattedArgument() / regularArgument()

    rule verbatimArgument() -> Document
      = "{{{" content:(anyContent()) "}}}" { vec![LitNode::Text(content)] }

    rule anyContent() -> String
      = content:$((!"}}}" [_])+) {
        String::from(content)
      }

    rule preformattedArgument() -> Document
      = "{{" content:(LitNode()*) "}}" { content }

    rule regularArgument() -> Document
      = "{" content:(LitNode()*) "}" { content }


    rule textContent() -> LitNode
      = content:$((
        [^ '\\' | '{' | '}']+ /
        "\\\\" /
        "\\{" /
        "\\}" /
        ("{" [^ '}']+ "}")
      )+) {
        LitNode::Text(String::from(content))
      }
  }
}

fn clean_text(text: &str) -> String {
  text
    .lines()
    .map(|t| t.trim())
    .collect::<Vec<_>>()
    .join("\n")
}

fn trim_codeblock(text: &str) -> String {
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

fn raw_text(nodes: &Vec<LitNode>) -> String {
  nodes
    .iter()
    .map(|n| match n {
      LitNode::Text(t) => t.as_str(),
      _ => "",
    })
    .collect::<String>()
}

fn text_to_markdown(nodes: &Vec<LitNode>) -> String {
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
}

pub fn transform_to_jsonschemas(doc: &Document) -> Vec<Schema> {
  doc
    .iter()
    .flat_map(|node| {
      match node {
        LitNode::Text(_) => {
          vec![]
        } // TODO: collect text here
        LitNode::Fn(schema, args) if (schema == "schema") => {
          let mut found_schemas: Vec<Schema> = vec![];

          let props = args[1]
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
                    type_name: type_name
                      .trim_start_matches("[")
                      .trim_end_matches("]")
                      .to_string(),
                    list: is_list,
                  },
                )]
              }
              _ => vec![], //panic!("Unexpected non-property function call in schema"),
            })
            .collect();

          found_schemas.push(Schema {
            name: text_to_markdown(&args[0]),
            properties: props,
          });

          return found_schemas;
        }
        _ => {
          vec![]
        }
      }
    })
    .collect()
}

fn parse(contents: &str) -> Result<Vec<Schema>, ParseError<LineCol>> {
  let lit_document = lit_parser::doc(&contents)?;
  Result::Ok(transform_to_jsonschemas(&lit_document))
}

/// Concourse documentation parser
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
  /// Path to the lit files to parse
  #[clap(value_parser)]
  litfiles: Vec<String>,

  /// Existing schema
  #[clap(short, long, default_value = "schema.json")]
  schema: String,
}

pub fn main() {
  let args = Args::parse();

  let schema_docs = args
    .litfiles
    .iter()
    .flat_map(|path| match parse(&fs::read_to_string(path).unwrap()) {
      Ok(doc) => doc,

      Err(e) => {
        println!("In {}", path);
        println!("{}", e);
        panic!("Unexpected parse error, aborting");
      }
    })
    .map(|schema| (schema.name.clone(), schema))
    .collect::<HashMap<_, _>>();

  let mut json_schema = json_parser::json(&fs::read_to_string(args.schema).unwrap()).unwrap();

  let definitions = match json_schema {
    Json::Object(ref mut o) => match o.get_mut("definitions").unwrap() {
      Json::Object(ref mut defs) => { defs }
      _ => {panic!() }
    }
    _ => { panic!() }
  };

  // definitions.into_iter().map(|(def_name, def)| {
  //   (def_name, def.
  // })
  for (definition_name, definition) in definitions {
    match schema_docs.get(&definition_name.to_lowercase()) {
      Some(schema_doc) => {
        match definition {
          Json::Object(defs) => {
            match defs.get_mut("properties").unwrap() {
              Json::Object(props) => {
                for (prop_name, prop) in props {
                  match prop {
                    Json::Object(prop_val) => {
                      let prop_doc = schema_doc.properties[prop_name].docs.clone();
                      prop_val.insert("description".to_string(), Json::String(prop_doc));
                    }
                    _ => { }
                  }
                }
              }
              _ => {}
            }
          }
          _ => {}
        }
      }
      None => {}
    }
  }

  let definitionsImm = &*definitions;
  println!("{:#?}\n\n{:#2?}", schema_docs, definitionsImm);
}
