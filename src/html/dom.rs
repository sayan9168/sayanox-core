//! Owned DOM nodes. Children live on elements and the document, so no parent
//! pointers or shared ownership are needed for this initial tree representation.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub children: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Element(Element),
    Text(String),
    Comment(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    pub tag_name: String,
    pub attributes: Vec<Attribute>,
    pub children: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub name: String,
    pub value: String,
}

impl Element {
    /// Look up the first attribute matching `name` (ASCII case-insensitive).
    pub fn get_attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|attr| attr.name.eq_ignore_ascii_case(name))
            .map(|attr| attr.value.as_str())
    }

    /// Return the element's `id` attribute if present and non-empty.
    pub fn id(&self) -> Option<&str> {
        self.get_attribute("id").filter(|id| !id.is_empty())
    }

    /// Iterate over whitespace-separated tokens in the `class` attribute.
    pub fn classes(&self) -> impl Iterator<Item = &str> {
        self.get_attribute("class")
            .unwrap_or("")
            .split_ascii_whitespace()
    }

    /// Check whether `class_name` appears in the element's `class` list.
    pub fn has_class(&self, class_name: &str) -> bool {
        self.classes().any(|cls| cls == class_name)
    }
}
