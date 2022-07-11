pub type LitDocument = Vec<LitNode>;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum LitNode {
  Text(String),
  Fn(String, Vec<LitDocument>),
  Comment(String),
}
