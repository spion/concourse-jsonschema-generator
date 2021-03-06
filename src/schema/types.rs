use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq)]
pub struct Schema {
  pub schema_name: String,
  pub is_group_member: bool,
  pub group_members: Vec<String>,
  pub properties: HashMap<String, Property>,
}
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Property {
  pub type_name: PropertyType,
  pub required: bool,
  pub list: bool,
  pub docs: String,
}
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum PropertyType {
  OneOf(Vec<PropertyType>),
  Constant(String),
  Ref(String),
  ArrayOf(Box<PropertyType>),
  Dict,
}
