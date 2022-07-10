use itertools::Itertools;
use peg::error::ParseError;
use peg::str::LineCol;
use std::collections::HashMap;
use std::fs;

use clap::Parser;

type Document = Vec<LitNode>;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum LitNode {
    Text(String),
    Fn(String, Vec<Document>),
    Comment(String),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Schema {
    schema_name: String,
    properties: HashMap<String, Property>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum PropertyType {
    OneOf(Vec<PropertyType>),
    Constant(String),
    Ref(String),
    ArrayOf(Box<PropertyType>),
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Property {
    type_name: PropertyType,
    required: bool,
    list: bool,
    docs: String,
}

#[derive(Debug, PartialEq, Eq)]
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
    text.lines()
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

    text.split("\n")
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
        .replace("\\{", "{")
        .replace("\\}", "}")
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn snake_to_pascal(s: &str) -> String {
    s.split("_").map(capitalize).collect()
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

pub fn transform_to_jsonschemas(doc: &Document) -> Vec<Schema> {
    doc.iter()
        .flat_map(|node| {
            match node {
                LitNode::Text(_) => {
                    vec![]
                } // TODO: collect text here
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
                        LitNode::Fn(other_fn, args) => {
                            found_schemas.extend(
                                args.into_iter()
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

pub fn update_schema(schema: &mut Json, schema_docs: &HashMap<String, Schema>) {
    let definitions = match schema {
        Json::Object(ref mut o) => match o.get_mut("definitions").unwrap() {
            Json::Object(ref mut defs) => defs,
            _ => {
                panic!()
            }
        },
        _ => {
            panic!()
        }
    };

    println!("Doc keys {:#?}", schema_docs.keys().collect::<Vec<_>>());
    println!("Def keys {:#?}", definitions.keys().collect::<Vec<_>>());

    for (definition_name, definition) in definitions {
        match schema_docs.get(definition_name) {
            Some(schema_doc) => {
                println!("Schema FOUND: {}", definition_name);
                match definition {
                    Json::Object(defs) => match defs.get_mut("properties") {
                        Some(Json::Object(props)) => {
                            for (prop_name, prop) in props {
                                match prop {
                                    Json::Object(prop_val) => {
                                        match schema_doc.properties.get(prop_name) {
                                            Some(prop_doc) => {
                                                // println!(
                                                //   "Prop docs found for schema {}, prop {}",
                                                //   definition_name, prop_name
                                                // );
                                                prop_val.insert(
                                                    "description".to_string(),
                                                    Json::String(prop_doc.docs.clone()),
                                                );
                                            }
                                            _ => {}
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
            _ => {
                println!("Schema not found: {}", definition_name);
            }
        }
    }
}

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

    // println!("{:#?}", schema_docs);

    let all_types = schema_docs
        .values()
        .flat_map(|schema_props| {
            schema_props
                .properties
                .values()
                .map(|prop_val| &prop_val.type_name)
        })
        .unique()
        .collect::<Vec<_>>();

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
                r#"{"type":"object","patternProperties":{".*":{"additionalProperties":true}}}"#
                    .to_string()
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
    // let type_names = schema_docs.flat_map(|doc| doc.properties.flat_map(|prop| prop.type_name));

    // let mut json_schema = json_parser::json(&fs::read_to_string(args.schema).unwrap()).unwrap();

    // update_schema(&mut json_schema, &schema_docs);

    // println!("{:#?}\n\n{:#2?}", "", json_schema);
}
