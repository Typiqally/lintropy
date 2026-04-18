//! Language registry for lintropy.
//!
//! Owns every `tree-sitter-*` grammar dependency. MVP only registers Rust (§13.1).

use tree_sitter::Language as TsLanguage;

/// Languages lintropy knows how to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    /// `tree-sitter-rust`.
    Rust,
    #[cfg(feature = "lang-go")]
    /// `tree-sitter-go`.
    Go,
    #[cfg(feature = "lang-python")]
    /// `tree-sitter-python`.
    Python,
}

impl Language {
    /// Resolve a `language:` YAML value (e.g. `"rust"`) to a [`Language`].
    pub fn from_name(name: &str) -> Option<Language> {
        match name {
            "rust" => Some(Language::Rust),
            #[cfg(feature = "lang-go")]
            "go" => Some(Language::Go),
            #[cfg(feature = "lang-python")]
            "python" => Some(Language::Python),
            _ => None,
        }
    }

    /// Resolve a file extension (without the leading dot) to a [`Language`].
    pub fn from_extension(ext: &str) -> Option<Language> {
        match ext {
            "rs" => Some(Language::Rust),
            #[cfg(feature = "lang-go")]
            "go" => Some(Language::Go),
            #[cfg(feature = "lang-python")]
            "py" | "pyi" => Some(Language::Python),
            _ => None,
        }
    }

    /// Canonical name as it should appear in a `language:` YAML value.
    pub fn name(self) -> &'static str {
        match self {
            Language::Rust => "rust",
            #[cfg(feature = "lang-go")]
            Language::Go => "go",
            #[cfg(feature = "lang-python")]
            Language::Python => "python",
        }
    }

    /// Default extensions associated with this language.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Language::Rust => &["rs"],
            #[cfg(feature = "lang-go")]
            Language::Go => &["go"],
            #[cfg(feature = "lang-python")]
            Language::Python => &["py", "pyi"],
        }
    }

    /// Native `tree-sitter` language handle for the grammar.
    ///
    /// `path` picks between multi-grammar languages (TypeScript's
    /// `typescript` vs `tsx`). Other variants ignore it.
    pub fn ts_language(self, _path: &std::path::Path) -> TsLanguage {
        match self {
            Language::Rust => tree_sitter_rust::language(),
            #[cfg(feature = "lang-go")]
            Language::Go => tree_sitter_go::language(),
            #[cfg(feature = "lang-python")]
            Language::Python => tree_sitter_python::language(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_name_round_trips() {
        assert_eq!(Language::from_name("rust"), Some(Language::Rust));
        assert_eq!(Language::from_name("ruby"), None);
        assert_eq!(Language::Rust.name(), "rust");
    }

    #[test]
    fn from_extension_rust() {
        assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
        #[cfg(not(feature = "lang-go"))]
        assert_eq!(Language::from_extension("go"), None);
        #[cfg(not(feature = "lang-python"))]
        assert_eq!(Language::from_extension("py"), None);
        assert!(Language::Rust.extensions().contains(&"rs"));
    }

    #[test]
    fn ts_language_loads() {
        let lang = Language::Rust.ts_language(std::path::Path::new("t.rs"));
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&lang).unwrap();
        let tree = parser.parse("fn main() {}", None).unwrap();
        assert_eq!(tree.root_node().kind(), "source_file");
    }

    #[cfg(feature = "lang-go")]
    #[test]
    fn from_name_resolves_go() {
        assert_eq!(Language::from_name("go"), Some(Language::Go));
        assert_eq!(Language::Go.name(), "go");
    }

    #[cfg(feature = "lang-go")]
    #[test]
    fn from_extension_resolves_go() {
        assert_eq!(Language::from_extension("go"), Some(Language::Go));
        assert!(Language::Go.extensions().contains(&"go"));
    }

    #[cfg(feature = "lang-go")]
    #[test]
    fn go_ts_language_parses_hello_world() {
        let lang = Language::Go.ts_language(std::path::Path::new("t.go"));
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&lang).unwrap();
        let tree = parser.parse("package main\nfunc main() {}", None).unwrap();
        assert_eq!(tree.root_node().kind(), "source_file");
    }

    #[cfg(feature = "lang-python")]
    #[test]
    fn from_name_resolves_python() {
        assert_eq!(Language::from_name("python"), Some(Language::Python));
        assert_eq!(Language::Python.name(), "python");
    }

    #[cfg(feature = "lang-python")]
    #[test]
    fn from_extension_resolves_python_and_pyi() {
        assert_eq!(Language::from_extension("py"), Some(Language::Python));
        assert_eq!(Language::from_extension("pyi"), Some(Language::Python));
        let exts = Language::Python.extensions();
        assert!(exts.contains(&"py"));
        assert!(exts.contains(&"pyi"));
    }

    #[cfg(feature = "lang-python")]
    #[test]
    fn python_ts_language_parses_hello_world() {
        let lang = Language::Python.ts_language(std::path::Path::new("t.py"));
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&lang).unwrap();
        let tree = parser.parse("def hi():\n    pass\n", None).unwrap();
        assert_eq!(tree.root_node().kind(), "module");
    }
}
