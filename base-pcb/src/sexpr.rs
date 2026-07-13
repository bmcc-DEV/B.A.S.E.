/// S-expression generation for KiCad file formats.
///
/// KiCad 7+/8+ stores schematics and PCBs as S-expressions.
/// This module provides building blocks for generating valid S-expressions.

/// A single S-expression node: `(name arg1 arg2 ...)`
#[derive(Debug, Clone)]
pub struct SExpr {
    pub name: String,
    pub args: Vec<SExprArg>,
}

#[derive(Debug, Clone)]
pub enum SExprArg {
    Atom(String),
    List(SExpr),
}

impl SExpr {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), args: Vec::new() }
    }

    pub fn atom(mut self, val: &str) -> Self {
        self.args.push(SExprArg::Atom(val.to_string()));
        self
    }

    pub fn int(self, val: i64) -> Self {
        self.atom(&val.to_string())
    }

    pub fn float(self, val: f64) -> Self {
        self.atom(&format!("{:.6}", val))
    }

    pub fn list(mut self, list: SExpr) -> Self {
        self.args.push(SExprArg::List(list));
        self
    }

    pub fn to_string(&self, depth: usize) -> String {
        let indent = "  ".repeat(depth);
        let mut s = format!("{}({}", indent, self.name);

        for arg in &self.args {
            match arg {
                SExprArg::Atom(a) => {
                    let val = if a.contains(' ') || a.contains('(') || a.contains('"') {
                        format!("\"{}\"", a)
                    } else {
                        a.clone()
                    };
                    s.push_str(&format!(" {}", val));
                }
                SExprArg::List(l) => {
                    s.push('\n');
                    s.push_str(&l.to_string(depth + 1));
                }
            }
        }

        s.push_str(")");
        s
    }

    /// Render as a single line (no indentation)
    pub fn to_string_flat(&self) -> String {
        let cap = 100 + self.args.len() * 20;
        let mut s = String::with_capacity(cap);
        s.push('(');
        s.push_str(&self.name);

        for arg in &self.args {
            match arg {
                SExprArg::Atom(a) => {
                    s.push(' ');
                    if a.contains(' ') || a.contains('(') || a.contains('"') {
                        s.push('"');
                        s.push_str(a);
                        s.push('"');
                    } else {
                        s.push_str(a);
                    }
                }
                SExprArg::List(l) => {
                    s.push(' ');
                    s.push_str(&l.to_string_flat());
                }
            }
        }

        s.push(')');
        s
    }
}

/// Shortcut for creating an S-expression
pub fn sexpr(name: &str) -> SExpr {
    SExpr::new(name)
}

/// Escape a string for use in an S-expression atom
pub fn escape(s: &str) -> String {
    if s.contains(' ') || s.contains('(') || s.contains(')') || s.contains('"') {
        format!("\"{}\"", s.replace('"', "\\\""))
    } else {
        s.to_string()
    }
}

/// Pretty-print a list of S-expressions as a KiCad file
pub fn render_kicad_file(header: &SExpr, body: &[SExpr]) -> String {
    let mut s = String::new();
    s.push_str(&header.to_string(0));
    s.push('\n');
    for expr in body {
        s.push_str(&expr.to_string(0));
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_sexpr() {
        let expr = sexpr("kicad_sch")
            .atom("(version 20231121)")
            .atom("(generator \"base-pcb\")");
        let s = expr.to_string(0);
        assert!(s.starts_with("(kicad_sch"));
        assert!(s.contains("version"));
    }

    #[test]
    fn test_nested_sexpr() {
        let inner = sexpr("font").atom("(size 1.27 1.27)");
        let outer = sexpr("property")
            .atom("Reference")
            .atom("R1")
            .list(inner);
        let s = outer.to_string_flat();
        assert!(s.contains("property"));
        assert!(s.contains("R1"));
    }

    #[test]
    fn test_escape() {
        assert_eq!(escape("hello"), "hello");
        assert_eq!(escape("hello world"), "\"hello world\"");
        assert_eq!(escape("GND"), "GND");
    }

    #[test]
    fn test_render_kicad_file() {
        let header = sexpr("kicad_sch")
            .atom("(version 20231121)")
            .atom("(generator \"base-pcb\")");
        let body = vec![
            sexpr("symbol").atom("R1").atom("(value 10k)"),
            sexpr("symbol").atom("C1").atom("(value 100n)"),
        ];
        let rendered = render_kicad_file(&header, &body);
        assert!(rendered.contains("R1"));
        assert!(rendered.contains("C1"));
    }
}
