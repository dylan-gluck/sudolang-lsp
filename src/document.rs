use tree_sitter::{Parser, Tree};

/// A tracked open document: source text + parsed tree.
pub struct Document {
    pub version: i32,
    pub text: String,
    pub tree: Tree,
}

impl Document {
    pub fn new(text: String, version: i32) -> Option<Self> {
        let mut parser = Parser::new();
        parser
            .set_language(&crate::sudolang_language())
            .expect("loading SudoLang grammar should not fail");
        let tree = parser.parse(&text, None)?;
        Some(Self { version, text, tree })
    }

    pub fn update(&mut self, text: String, version: i32) {
        self.text = text;
        self.version = version;
        let mut parser = Parser::new();
        parser
            .set_language(&crate::sudolang_language())
            .expect("loading SudoLang grammar should not fail");
        if let Some(tree) = parser.parse(&self.text, None) {
            self.tree = tree;
        }
    }
}
