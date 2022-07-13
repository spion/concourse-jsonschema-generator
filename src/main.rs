use clap::Parser;
use std::collections::HashMap;
use std::fs;

mod convert;
mod lit;
mod schema;

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
    .flat_map(|path| {
      let contents = fs::read_to_string(path).unwrap();
      let lit_document = lit::parse(&contents);
      match lit_document {
        Ok(doc) => convert::to_jsonschemas(&doc),

        Err(e) => {
          eprintln!("In {}", path);
          eprintln!("{}", e);
          panic!("Unexpected parse error, aborting");
        }
      }
    })
    .map(|schema| (schema.schema_name.clone(), schema))
    .collect::<HashMap<_, _>>();

  let schema = schema::serialize::serialize(&schema_docs);

  println!("{}", schema);
}
