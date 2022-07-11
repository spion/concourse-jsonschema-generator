pub mod types;

use peg::{error::ParseError, str::LineCol};
use types::{LitDocument, LitNode};

peg::parser! {
  grammar lit_parser() for str {
    pub rule doc() -> LitDocument
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

    rule argument() -> LitDocument
      = verbatimArgument() / preformattedArgument() / regularArgument()

    rule verbatimArgument() -> LitDocument
      = "{{{" content:(anyContent()) "}}}" { vec![LitNode::Text(content)] }

    rule anyContent() -> String
      = content:$((!"}}}" [_])+) {
        String::from(content)
      }

    rule preformattedArgument() -> LitDocument
      = "{{" content:(LitNode()*) "}}" { content }

    rule regularArgument() -> LitDocument
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

pub fn parse(contents: &str) -> Result<LitDocument, ParseError<LineCol>> {
  lit_parser::doc(&contents)
}
