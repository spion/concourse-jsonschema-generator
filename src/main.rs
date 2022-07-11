use itertools::Itertools;
use peg::error::ParseError;
use peg::str::LineCol;
use schema::types::Schema;
use std::collections::HashMap;
use std::fs;

use clap::Parser;

use schema::serialize::prop_type_to_jsonschema;

mod convert;
mod lit;
mod schema;

fn parse(contents: &str) -> Result<Vec<Schema>, ParseError<LineCol>> {
  let lit_document = lit::parse(&contents)?;
  Result::Ok(convert::transform_to_jsonschemas(&lit_document))
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

  // Typename mappings:
  // config -> match everything
  //
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
    .map(|schema| (schema.schema_name.clone(), schema))
    .collect::<HashMap<_, _>>();

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

      let schema_obj = if schema_props.len() > 0 {
        format!(
          r#"{{"additionalProperties":false,"type":"object","properties":{{{}}}}}"#,
          schema_props.join(",")
        )
      } else if schema_name == "number" {
        r#"{"type":"number"}"#.to_string()
      } else if schema_name == "boolean" {
        r#"{"type":"boolean"}"#.to_string()
      } else if schema_name == "config" || schema_name == "value" {
        r#"{"type":"object","patternProperties":{".*":{"additionalProperties":true}}}"#.to_string()
      } else {
        r#"{"type":"string"}"#.to_string()
      };

      format!(r#""{}":{}"#, schema_name, schema_obj)
    })
    .join(",");

  let schema = format!(
    r###"
    {{
      "$schema": "http://json-schema.org/draft-04/schema#",
      "$ref": "#/definitions/pipeline",
      "additionalProperties": true,
      "definitions":{{{}}}
    }}
    "###,
    definitions
  );

  println!("{}", schema);
}
