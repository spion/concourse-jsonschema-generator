use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq)]
pub struct Schema {
  pub schema_name: String,
  pub part_of_group: bool,
  pub group_members: Vec<String>,
  pub properties: HashMap<String, Property>,
}
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Property {
  pub type_name: PropertyType,
  pub required: bool,
  pub list: bool,
  pub docs: String,
}
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum PropertyType {
  OneOf(Vec<PropertyType>),
  Constant(String),
  Ref(String),
  ArrayOf(Box<PropertyType>),
  Dict,
}
