use std::path::Path;

use tree_sitter::Parser;

#[derive(Debug, Clone)]
pub struct SymbolEntry {
    pub name: String,
    pub qualified: Option<String>,
}

pub struct SymbolTable {
    entries: Vec<SymbolEntry>,
}

impl SymbolTable {
    /// Match by both qualified ("User.new") and unqualified ("new") names.
    pub fn contains(&self, name: &str) -> bool {
        self.entries.iter().any(|e| {
            e.name == name || e.qualified.as_deref() == Some(name)
        })
    }

    /// Find suggestions when a symbol isn't found.
    /// Tier 1: leaf name matches → suggest qualified form
    /// Tier 2: case-insensitive match on any matchable name
    /// Tier 3: prefix match on any matchable name
    pub fn suggest(&self, name: &str) -> Vec<String> {
        // Tier 1: user typed a leaf name that exists as a qualified entry
        let qualified: Vec<String> = self
            .entries
            .iter()
            .filter(|e| e.name == name && e.qualified.is_some())
            .map(|e| e.qualified.clone().unwrap())
            .collect();
        if !qualified.is_empty() {
            return qualified;
        }

        // Tier 2: case-insensitive match
        let lower = name.to_lowercase();
        let case_matches: Vec<String> = self
            .entries
            .iter()
            .flat_map(|e| {
                let mut v = Vec::new();
                if e.name.to_lowercase() == lower {
                    v.push(e.qualified.as_ref().unwrap_or(&e.name).clone());
                }
                if let Some(ref q) = e.qualified
                    && q.to_lowercase() == lower
                {
                    v.push(q.clone());
                }
                v
            })
            .collect();
        if !case_matches.is_empty() {
            return dedup_stable(case_matches);
        }

        // Tier 3: prefix match
        let prefix_matches: Vec<String> = self
            .entries
            .iter()
            .flat_map(|e| {
                let mut v = Vec::new();
                if e.name.starts_with(name) {
                    v.push(e.qualified.as_ref().unwrap_or(&e.name).clone());
                }
                if let Some(ref q) = e.qualified
                    && q.starts_with(name)
                {
                    v.push(q.clone());
                }
                v
            })
            .collect();
        dedup_stable(prefix_matches)
    }

    /// Format symbols grouped by container: "User { new, create }, validate_email"
    pub fn format_hint(&self) -> String {
        // First pass: identify all parent names from qualified entries
        let parent_names: std::collections::HashSet<String> = self
            .entries
            .iter()
            .filter_map(|e| {
                e.qualified
                    .as_ref()
                    .and_then(|q| q.split_once('.'))
                    .map(|(p, _)| p.to_string())
            })
            .collect();

        // Second pass: build groups in insertion order
        let mut groups: Vec<(Option<String>, Vec<String>)> = Vec::new();

        for entry in &self.entries {
            if let Some(ref q) = entry.qualified {
                let parent = q.split_once('.').map(|(p, _)| p.to_string()).unwrap_or_default();
                if let Some(group) = groups.iter_mut().find(|(p, _)| *p == Some(parent.clone())) {
                    group.1.push(entry.name.clone());
                } else {
                    groups.push((Some(parent), vec![entry.name.clone()]));
                }
            } else if !parent_names.contains(&entry.name) {
                // Top-level symbol that isn't a container — add as standalone
                // (avoid duplicate standalone entries)
                let already = groups.iter().any(|(p, children)| {
                    p.is_none() && children.contains(&entry.name)
                });
                if !already {
                    groups.push((None, vec![entry.name.clone()]));
                }
            }
        }

        groups
            .iter()
            .map(|(parent, children)| {
                if let Some(p) = parent {
                    format!("{p} {{ {} }}", children.join(", "))
                } else {
                    children[0].clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Flat list of all matchable names (for backward-compatible assertions).
    #[allow(dead_code)]
    pub fn names(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .entries
            .iter()
            .flat_map(|e| {
                let mut v = vec![e.name.clone()];
                if let Some(ref q) = e.qualified {
                    v.push(q.clone());
                }
                v
            })
            .collect();
        out.sort();
        out.dedup();
        out
    }
}

fn dedup_stable(mut v: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    v.retain(|s| seen.insert(s.clone()));
    v
}

/// Extract symbols from a source file using tree-sitter.
///
/// Returns `None` if the language is unsupported (no grammar available).
pub fn extract_symbols(file_path: &Path, source: &str) -> Option<SymbolTable> {
    let lang = detect_language(file_path)?;
    let grammar = lang.grammar();

    let mut parser = Parser::new();
    parser
        .set_language(&grammar.into())
        .expect("grammar version mismatch");

    let tree = parser.parse(source, None)?;
    let root = tree.root_node();

    let mut entries = Vec::new();
    walk(root, None, source, lang, &mut entries);

    Some(SymbolTable { entries })
}

/// Walk the AST recursively, collecting symbol entries with parent context.
fn walk(
    node: tree_sitter::Node,
    parent_name: Option<&str>,
    source: &str,
    lang: Lang,
    out: &mut Vec<SymbolEntry>,
) {
    let kind = node.kind();

    if let Some(def) = classify_node(kind, lang) {
        let name = extract_name(&node, source, lang);
        if let Some(name) = name {
            out.push(SymbolEntry {
                name: name.clone(),
                qualified: parent_name.map(|p| format!("{p}.{name}")),
            });

            if def.is_container {
                let cursor = &mut node.walk();
                for child in node.children(cursor) {
                    walk(child, Some(&name), source, lang, out);
                }
                return;
            }
        }
    }

    let cursor = &mut node.walk();
    for child in node.children(cursor) {
        walk(child, parent_name, source, lang, out);
    }
}

/// What we learned about a node's definition role.
struct DefInfo {
    is_container: bool,
}

/// Classify a node kind for the given language.
fn classify_node(kind: &str, lang: Lang) -> Option<DefInfo> {
    match lang {
        Lang::Rust => classify_rust(kind),
        Lang::TypeScript | Lang::JavaScript => classify_ts_js(kind),
        Lang::Python => classify_python(kind),
        Lang::Go => classify_go(kind),
    }
}

fn classify_rust(kind: &str) -> Option<DefInfo> {
    match kind {
        "function_item" | "const_item" | "static_item" | "type_item" => {
            Some(DefInfo { is_container: false })
        }
        "struct_item" | "enum_item" | "trait_item" | "mod_item" => {
            Some(DefInfo { is_container: true })
        }
        "impl_item" => Some(DefInfo { is_container: true }),
        _ => None,
    }
}

fn classify_ts_js(kind: &str) -> Option<DefInfo> {
    match kind {
        "function_declaration" | "method_definition" | "variable_declarator"
        | "type_alias_declaration" => Some(DefInfo { is_container: false }),
        "class_declaration" | "interface_declaration" | "enum_declaration" => {
            Some(DefInfo { is_container: true })
        }
        _ => None,
    }
}

fn classify_python(kind: &str) -> Option<DefInfo> {
    match kind {
        "function_definition" => Some(DefInfo { is_container: false }),
        "class_definition" => Some(DefInfo { is_container: true }),
        _ => None,
    }
}

fn classify_go(kind: &str) -> Option<DefInfo> {
    match kind {
        "function_declaration" | "method_declaration" | "type_spec" => {
            Some(DefInfo { is_container: false })
        }
        _ => None,
    }
}

/// Extract the name of a definition node, handling language-specific quirks.
fn extract_name(node: &tree_sitter::Node, source: &str, lang: Lang) -> Option<String> {
    let kind = node.kind();

    // Rust impl_item: name comes from the "type" field, not "name"
    if lang == Lang::Rust && kind == "impl_item" {
        return node
            .child_by_field_name("type")
            .map(|n| text(n, source).to_string());
    }

    // Go method_declaration: emit as ReceiverType.MethodName
    // The name is just the method name; receiver context handled by caller
    // Actually, Go methods aren't nested in a container — we need special handling
    if lang == Lang::Go && kind == "method_declaration" {
        let method_name = node.child_by_field_name("name")?;
        let receiver_type = extract_go_receiver_type(node, source);
        if let Some(recv) = receiver_type {
            return Some(format!("{recv}.{}", text(method_name, source)));
        }
        return Some(text(method_name, source).to_string());
    }

    node.child_by_field_name("name")
        .map(|n| text(n, source).to_string())
}

/// Extract the receiver type from a Go method_declaration.
/// e.g., `func (s *MyStruct) Method()` → "MyStruct"
fn extract_go_receiver_type(node: &tree_sitter::Node, source: &str) -> Option<String> {
    let receiver = node.child_by_field_name("receiver")?;
    // receiver is a parameter_list containing parameter_declaration(s)
    let cursor = &mut receiver.walk();
    for child in receiver.children(cursor) {
        if child.kind() == "parameter_declaration" {
            let type_node = child.child_by_field_name("type")?;
            return Some(unwrap_go_type(type_node, source));
        }
    }
    None
}

/// Unwrap pointer/slice types to get the base type name.
fn unwrap_go_type(node: tree_sitter::Node, source: &str) -> String {
    match node.kind() {
        "pointer_type" => {
            // *T → get T
            let cursor = &mut node.walk();
            for child in node.children(cursor) {
                if child.kind() != "*" {
                    return unwrap_go_type(child, source);
                }
            }
            text(node, source).to_string()
        }
        _ => text(node, source).to_string(),
    }
}

fn text<'a>(node: tree_sitter::Node, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}

// -- Language detection ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lang {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
}

impl Lang {
    fn grammar(self) -> tree_sitter_language::LanguageFn {
        match self {
            Lang::Rust => tree_sitter_rust::LANGUAGE,
            Lang::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
            Lang::JavaScript => tree_sitter_javascript::LANGUAGE,
            Lang::Python => tree_sitter_python::LANGUAGE,
            Lang::Go => tree_sitter_go::LANGUAGE,
        }
    }
}

fn detect_language(path: &Path) -> Option<Lang> {
    match path.extension()?.to_str()? {
        "rs" => Some(Lang::Rust),
        "ts" | "tsx" => Some(Lang::TypeScript),
        "js" | "jsx" => Some(Lang::JavaScript),
        "py" => Some(Lang::Python),
        "go" => Some(Lang::Go),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn symbols(ext: &str, source: &str) -> Vec<String> {
        let path = PathBuf::from(format!("test.{ext}"));
        extract_symbols(&path, source).unwrap().names()
    }

    // -- Rust --

    #[test]
    fn rust_top_level_fn() {
        let syms = symbols("rs", "fn validate_email() {}");
        assert!(syms.contains(&"validate_email".to_string()));
    }

    #[test]
    fn rust_struct_and_enum() {
        let syms = symbols("rs", "struct User {}\nenum Status { Active }");
        assert!(syms.contains(&"User".to_string()));
        assert!(syms.contains(&"Status".to_string()));
    }

    #[test]
    fn rust_impl_method_qualified() {
        let syms = symbols(
            "rs",
            "struct MyStruct {}\nimpl MyStruct {\n    fn method(&self) {}\n}",
        );
        assert!(syms.contains(&"MyStruct".to_string()));
        assert!(syms.contains(&"MyStruct.method".to_string()));
        assert!(syms.contains(&"method".to_string()));
    }

    #[test]
    fn rust_const_static_type() {
        let syms = symbols(
            "rs",
            "const MAX: u32 = 100;\nstatic GLOBAL: &str = \"hi\";\ntype Alias = Vec<u8>;",
        );
        assert!(syms.contains(&"MAX".to_string()));
        assert!(syms.contains(&"GLOBAL".to_string()));
        assert!(syms.contains(&"Alias".to_string()));
    }

    // -- TypeScript --

    #[test]
    fn typescript_class_with_method() {
        let syms = symbols("ts", "class UserService {\n  create() {}\n}");
        assert!(syms.contains(&"UserService".to_string()));
        assert!(syms.contains(&"UserService.create".to_string()));
        assert!(syms.contains(&"create".to_string()));
    }

    #[test]
    fn typescript_function_declaration() {
        let syms = symbols("ts", "function greet(name: string) {}");
        assert!(syms.contains(&"greet".to_string()));
    }

    // -- JavaScript --

    #[test]
    fn javascript_class_with_method() {
        let syms = symbols("js", "class Foo {\n  bar() {}\n}");
        assert!(syms.contains(&"Foo".to_string()));
        assert!(syms.contains(&"Foo.bar".to_string()));
    }

    // -- Python --

    #[test]
    fn python_class_with_method() {
        let syms = symbols("py", "class Foo:\n    def bar(self):\n        pass");
        assert!(syms.contains(&"Foo".to_string()));
        assert!(syms.contains(&"Foo.bar".to_string()));
        assert!(syms.contains(&"bar".to_string()));
    }

    #[test]
    fn python_top_level_function() {
        let syms = symbols("py", "def main():\n    pass");
        assert!(syms.contains(&"main".to_string()));
    }

    // -- Go --

    #[test]
    fn go_function_and_type() {
        let syms = symbols(
            "go",
            "package main\n\nfunc main() {}\n\ntype Foo struct{}",
        );
        assert!(syms.contains(&"main".to_string()));
        assert!(syms.contains(&"Foo".to_string()));
    }

    #[test]
    fn go_method_with_receiver() {
        let syms = symbols(
            "go",
            "package main\n\ntype Foo struct{}\n\nfunc (f *Foo) Bar() {}",
        );
        assert!(syms.contains(&"Foo".to_string()));
        assert!(syms.contains(&"Foo.Bar".to_string()));
    }

    // -- Edge cases --

    #[test]
    fn unsupported_extension_returns_none() {
        let path = PathBuf::from("data.csv");
        assert!(extract_symbols(&path, "some,csv,data").is_none());
    }

    #[test]
    fn no_extension_returns_none() {
        let path = PathBuf::from("Makefile");
        assert!(extract_symbols(&path, "all: build").is_none());
    }

    // -- SymbolTable method tests --

    fn table(ext: &str, source: &str) -> SymbolTable {
        let path = PathBuf::from(format!("test.{ext}"));
        extract_symbols(&path, source).unwrap()
    }

    #[test]
    fn contains_qualified_name() {
        let t = table("rs", "struct User {}\nimpl User {\n    fn new() -> Self { User {} }\n}");
        assert!(t.contains("User.new"));
    }

    #[test]
    fn contains_unqualified_name() {
        let t = table("rs", "struct User {}\nimpl User {\n    fn new() -> Self { User {} }\n}");
        assert!(t.contains("new"));
    }

    #[test]
    fn contains_nonexistent() {
        let t = table("rs", "struct User {}\nimpl User {\n    fn new() -> Self { User {} }\n}");
        assert!(!t.contains("fake"));
    }

    #[test]
    fn suggest_qualified_form() {
        let t = table("rs", "struct User {}\nimpl User {\n    fn new() -> Self { User {} }\n}");
        let suggestions = t.suggest("new");
        assert_eq!(suggestions, vec!["User.new"]);
    }

    #[test]
    fn suggest_case_insensitive() {
        let t = table("rs", "struct User {}");
        let suggestions = t.suggest("user");
        assert_eq!(suggestions, vec!["User"]);
    }

    #[test]
    fn suggest_prefix() {
        let t = table("rs", "fn validate_email() {}");
        let suggestions = t.suggest("valid");
        assert_eq!(suggestions, vec!["validate_email"]);
    }

    #[test]
    fn suggest_no_match() {
        let t = table("rs", "fn validate_email() {}");
        let suggestions = t.suggest("zzz");
        assert!(suggestions.is_empty());
    }

    #[test]
    fn format_hint_groups_containers() {
        let t = table(
            "rs",
            "struct User {}\nimpl User {\n    fn new() -> Self { User {} }\n}\nfn validate_email() {}",
        );
        assert_eq!(t.format_hint(), "User { new }, validate_email");
    }

    #[test]
    fn format_hint_top_level_only() {
        let t = table("rs", "fn foo() {}\nfn bar() {}");
        assert_eq!(t.format_hint(), "foo, bar");
    }
}
