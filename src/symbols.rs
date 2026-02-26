use std::path::Path;

use tree_sitter::Parser;

/// Extract qualified symbol names from a source file using tree-sitter.
///
/// Returns `None` if the language is unsupported (no grammar available).
/// Returns `Some(vec)` with both qualified (`User.new`) and unqualified (`new`) names.
pub fn extract_symbols(file_path: &Path, source: &str) -> Option<Vec<String>> {
    let lang = detect_language(file_path)?;
    let grammar = lang.grammar();

    let mut parser = Parser::new();
    parser
        .set_language(&grammar.into())
        .expect("grammar version mismatch");

    let tree = parser.parse(source, None)?;
    let root = tree.root_node();

    let mut symbols = Vec::new();
    walk(root, None, source, lang, &mut symbols);

    symbols.sort();
    symbols.dedup();
    Some(symbols)
}

/// Walk the AST recursively, building qualified symbol names.
fn walk(
    node: tree_sitter::Node,
    parent_name: Option<&str>,
    source: &str,
    lang: Lang,
    out: &mut Vec<String>,
) {
    let kind = node.kind();

    if let Some(def) = classify_node(kind, lang) {
        let name = extract_name(&node, source, lang);
        if let Some(name) = name {
            // Emit qualified name if we have a parent
            if let Some(parent) = parent_name {
                out.push(format!("{parent}.{name}"));
            }
            // Always emit the unqualified name
            out.push(name.clone());

            // If this is a container, recurse with this as parent
            if def.is_container {
                let cursor = &mut node.walk();
                for child in node.children(cursor) {
                    walk(child, Some(&name), source, lang, out);
                }
                return;
            }
        }
    }

    // Non-definition node or leaf definition: recurse with same parent
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
        extract_symbols(&path, source).unwrap()
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
}
