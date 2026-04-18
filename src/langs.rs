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
    #[cfg(feature = "lang-typescript")]
    /// `tree-sitter-typescript`.
    TypeScript,
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
            #[cfg(feature = "lang-typescript")]
            "typescript" => Some(Language::TypeScript),
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
            #[cfg(feature = "lang-typescript")]
            "ts" | "tsx" | "mts" | "cts" | "d.ts" => Some(Language::TypeScript),
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
            #[cfg(feature = "lang-typescript")]
            Language::TypeScript => "typescript",
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
            #[cfg(feature = "lang-typescript")]
            Language::TypeScript => &["ts", "tsx", "mts", "cts", "d.ts"],
        }
    }

    /// Native `tree-sitter` language handle for the grammar.
    ///
    /// `path` picks between multi-grammar languages (TypeScript's
    /// `typescript` vs `tsx`). Other variants ignore it.
    pub fn ts_language(self, path: &std::path::Path) -> TsLanguage {
        match self {
            Language::Rust => {
                let _ = path;
                tree_sitter_rust::language()
            }
            #[cfg(feature = "lang-go")]
            Language::Go => {
                let _ = path;
                tree_sitter_go::language()
            }
            #[cfg(feature = "lang-python")]
            Language::Python => {
                let _ = path;
                tree_sitter_python::language()
            }
            #[cfg(feature = "lang-typescript")]
            Language::TypeScript => {
                if is_tsx_path(path) {
                    tree_sitter_typescript::language_tsx()
                } else {
                    tree_sitter_typescript::language_typescript()
                }
            }
        }
    }
}

/// Resolve a path to a [`Language`], handling the `.d.ts` compound suffix.
pub fn language_from_path(path: &std::path::Path) -> Option<Language> {
    let name = path.file_name()?.to_str()?;
    if name.ends_with(".d.ts") {
        return Language::from_extension("d.ts");
    }
    let ext = path.extension()?.to_str()?;
    Language::from_extension(ext)
}

#[cfg(feature = "lang-typescript")]
fn is_tsx_path(path: &std::path::Path) -> bool {
    path.extension().and_then(|e| e.to_str()) == Some("tsx")
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

    #[cfg(feature = "lang-typescript")]
    #[test]
    fn from_name_resolves_typescript() {
        assert_eq!(
            Language::from_name("typescript"),
            Some(Language::TypeScript)
        );
        assert_eq!(Language::TypeScript.name(), "typescript");
        // No `tsx` alias — rule authors use `typescript` for both.
        assert_eq!(Language::from_name("tsx"), None);
    }

    #[cfg(feature = "lang-typescript")]
    #[test]
    fn from_extension_resolves_typescript_family() {
        for ext in ["ts", "tsx", "mts", "cts"] {
            assert_eq!(
                Language::from_extension(ext),
                Some(Language::TypeScript),
                "extension: {ext}"
            );
        }
    }

    #[cfg(feature = "lang-typescript")]
    #[test]
    fn language_from_path_handles_d_ts_compound_suffix() {
        use std::path::Path;
        assert_eq!(
            language_from_path(Path::new("src/types.d.ts")),
            Some(Language::TypeScript)
        );
        assert_eq!(
            language_from_path(Path::new("src/api.ts")),
            Some(Language::TypeScript)
        );
        assert_eq!(
            language_from_path(Path::new("src/app.tsx")),
            Some(Language::TypeScript)
        );
        assert_eq!(
            language_from_path(Path::new("src/lib.rs")),
            Some(Language::Rust)
        );
        assert_eq!(language_from_path(Path::new("src/no-ext")), None);
    }

    #[cfg(feature = "lang-typescript")]
    #[test]
    fn typescript_dispatch_picks_tsx_grammar_for_tsx_ext() {
        use std::path::Path;
        let lang = Language::TypeScript.ts_language(Path::new("f.tsx"));
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&lang).unwrap();
        let tree = parser.parse("const x = <Foo/>;", None).unwrap();
        assert!(
            !has_error(tree.root_node()),
            "tsx grammar should parse JSX: {}",
            tree.root_node().to_sexp()
        );
    }

    #[cfg(feature = "lang-typescript")]
    #[test]
    fn typescript_dispatch_picks_typescript_grammar_for_ts_ext() {
        use std::path::Path;
        let lang = Language::TypeScript.ts_language(Path::new("f.ts"));
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&lang).unwrap();
        let tree = parser.parse("const x = <number>42;", None).unwrap();
        assert!(
            !has_error(tree.root_node()),
            "typescript grammar should parse type assertions: {}",
            tree.root_node().to_sexp()
        );
    }

    #[cfg(feature = "lang-typescript")]
    #[test]
    fn typescript_dispatch_picks_typescript_for_d_ts() {
        use std::path::Path;
        let lang = Language::TypeScript.ts_language(Path::new("types.d.ts"));
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&lang).unwrap();
        let tree = parser.parse("type T = <U>(x: U) => U;", None).unwrap();
        assert!(!has_error(tree.root_node()));
    }

    #[cfg(feature = "lang-typescript")]
    fn has_error(node: tree_sitter::Node) -> bool {
        if node.is_error() {
            return true;
        }
        for i in 0..node.child_count() {
            if has_error(node.child(i).unwrap()) {
                return true;
            }
        }
        false
    }
}
