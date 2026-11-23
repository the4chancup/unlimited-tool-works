//! Finds the mutants `cargo-mutants` never makes: `if` guards dropped by
//! replacing the condition with `false`, elements removed from the literal lists a
//! `matches!` or a separator method decides by, `.trim()`-style calls stripped,
//! in-place ordering statements deleted, any other method-call statement deleted
//! (`call`, on trial) and `true`/`false` arguments flipped.
//!
//! A mutant is a text patch, so positions come from `proc_macro2::Span`'s line and column
//! (columns count chars, not bytes) and are converted to byte offsets against the source.

use proc_macro2::{LineColumn, Span};
use syn::parse::{ParseStream, Parser};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{
    Attribute, Expr, ExprArray, ExprCall, ExprIf, ExprLit, ExprMacro, ExprMethodCall, ExprStruct,
    ImplItemFn, Item, ItemFn, Lit, Pat, PatOr, Stmt, Token, TraitItemFn,
};

/// The method names whose receiver or arguments decide by a literal list.
const LIST_METHODS: &[&str] = &[
    "split",
    "rsplit",
    "split_terminator",
    "splitn",
    "split_once",
    "rsplit_once",
    "contains",
    "starts_with",
    "ends_with",
    "trim_matches",
    "trim_start_matches",
    "trim_end_matches",
    "strip_prefix",
    "strip_suffix",
    "find",
    "rfind",
    "matches",
];

/// The no-argument spelling fixers a mutant drops to test whether a call site relied on
/// the spelling being tidied first.
const NORMALIZE_METHODS: &[&str] = &[
    "trim",
    "trim_start",
    "trim_end",
    "to_ascii_lowercase",
    "to_ascii_uppercase",
    "to_lowercase",
    "to_uppercase",
];

/// The in-place ordering methods whose statement a mutant deletes to test whether the
/// order they impose is asserted.
const ORDER_METHODS: &[&str] = &[
    "sort",
    "sort_unstable",
    "sort_by",
    "sort_by_key",
    "sort_unstable_by",
    "sort_unstable_by_key",
    "dedup",
    "dedup_by",
    "dedup_by_key",
    "reverse",
];

/// One mutant: replacing `text[start..end]` with `replacement` must still parse. `line` is
/// 1-based and `kind` is `guard`, `literal`, `normalize`, `order` or `bool`, as one row of
/// the lister's output is built.
pub(crate) struct Mutant {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) kind: &'static str,
    pub(crate) line: usize,
    pub(crate) replacement: String,
    pub(crate) description: String,
}

/// Every mutant in one source file's text, in source order.
pub(crate) fn scan_source(text: &str) -> Result<Vec<Mutant>, syn::Error> {
    // syn::parse_file strips a BOM and a shebang line before parsing; doing the same here
    // keeps the spans and this text the same length, so byte offsets line up.
    let content = strip_prelude(text);
    let file = syn::parse_file(content)?;
    let mut lister = Lister {
        text: content,
        line_starts: line_starts(content),
        fn_name: String::new(),
        mutants: Vec::new(),
    };
    lister.visit_file(&file);
    let skipped = text.len() - content.len();
    for mutant in &mut lister.mutants {
        mutant.start += skipped;
        mutant.end += skipped;
    }
    Ok(lister.mutants)
}

fn strip_prelude(text: &str) -> &str {
    let content = text.strip_prefix('\u{feff}').unwrap_or(text);
    if let Some(rest) = content.strip_prefix("#!")
        && !rest.trim_start().starts_with('[')
    {
        let end = content.find('\n').map_or(content.len(), |i| i + 1);
        return &content[end..];
    }
    content
}

/// The byte offset each 1-based line starts at, for line/column to byte conversion.
fn line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

/// The byte offset of a 1-based `line`, 0-based `column` position, `column` counting chars
/// as `proc_macro2::Span::start`/`end` report it.
fn byte_offset(text: &str, line_starts: &[usize], loc: LineColumn) -> usize {
    let base = line_starts[loc.line - 1];
    match text[base..].char_indices().nth(loc.column) {
        Some((offset, _)) => base + offset,
        None => text.len(),
    }
}

struct Lister<'a> {
    text: &'a str,
    line_starts: Vec<usize>,
    /// The function being visited, for the description. Empty at top level.
    fn_name: String,
    mutants: Vec<Mutant>,
}

impl Lister<'_> {
    fn start_of(&self, span: Span) -> usize {
        byte_offset(self.text, &self.line_starts, span.start())
    }

    fn end_of(&self, span: Span) -> usize {
        byte_offset(self.text, &self.line_starts, span.end())
    }

    /// The source text a byte range covers, whitespace collapsed so it can sit in a
    /// tab-separated line.
    fn snippet(&self, start: usize, end: usize) -> String {
        self.text[start..end]
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn description(&self, change: &str, start: usize, end: usize) -> String {
        let removed = self.snippet(start, end);
        let place = if self.fn_name.is_empty() {
            "at top level".to_owned()
        } else {
            format!("in {}", self.fn_name)
        };
        format!("{place}: {change} | removed: {removed}")
    }

    fn push(
        &mut self,
        kind: &'static str,
        start: usize,
        end: usize,
        replacement: impl Into<String>,
        change: &str,
        line: usize,
    ) {
        let description = self.description(change, start, end);
        self.mutants.push(Mutant {
            start,
            end,
            kind,
            line,
            replacement: replacement.into(),
            description,
        });
    }

    /// One removal range per element of a comma- or `|`-separated list: the element and the
    /// separator after it, or the separator before it for a last element with none after.
    /// `pairs` is `(element span, separator span)` per element, in order.
    fn removal_ranges(&self, pairs: &[(Span, Option<Span>)]) -> Vec<(usize, usize)> {
        pairs
            .iter()
            .enumerate()
            .map(|(i, &(element, separator))| match separator {
                Some(separator) => (self.start_of(element), self.end_of(separator)),
                None => (
                    self.start_of(
                        pairs[i - 1]
                            .1
                            .expect("every element but the last is followed by a separator"),
                    ),
                    self.end_of(element),
                ),
            })
            .collect()
    }

    /// The mutants of a `matches!(expr, pat)` call: one per alternative of a top-level
    /// or-pattern. Anything that does not parse as `expr, pat` is not a `matches!` this
    /// tool understands and is skipped.
    fn matches_mutants(&mut self, mac: &syn::Macro) {
        let Ok(Pat::Or(PatOr { cases, .. })) = matches_args.parse2(mac.tokens.clone()) else {
            return;
        };
        if cases.len() < 2 {
            return;
        }
        let pairs: Vec<(Span, Option<Span>)> = cases
            .pairs()
            .map(|pair| {
                (
                    pair.value().span(),
                    pair.punct().map(|token| token.spans[0]),
                )
            })
            .collect();
        let ranges = self.removal_ranges(&pairs);
        for (start, end) in ranges {
            let line = self.line_of(start);
            self.push(
                "literal",
                start,
                end,
                "",
                "the matches! pattern loses an alternative",
                line,
            );
        }
    }

    /// The mutants of `[a, b, ...]` used by a list-deciding method call: one per
    /// literal element. A non-literal element (a path, a constant) gets no
    /// removal, but does not stop the literal ones from listing theirs.
    fn array_mutants(&mut self, expr: &Expr, method: &str) {
        let mut expr = expr;
        while let Expr::Reference(reference) = expr {
            expr = &reference.expr;
        }
        let Expr::Array(ExprArray { elems, .. }) = expr else {
            return;
        };
        if elems.len() < 2 {
            return;
        }
        let pairs: Vec<(Span, Option<Span>)> = elems
            .pairs()
            .map(|pair| {
                (
                    pair.value().span(),
                    pair.punct().map(|token| token.spans[0]),
                )
            })
            .collect();
        let ranges = self.removal_ranges(&pairs);
        for ((start, end), element) in ranges.into_iter().zip(elems.iter()) {
            if !is_literal(element) {
                continue;
            }
            let line = self.line_of(start);
            let change = format!("the {method}() list loses an element");
            self.push("literal", start, end, "", &change, line);
        }
    }

    fn line_of(&self, byte: usize) -> usize {
        self.line_starts.partition_point(|&start| start <= byte)
    }

    /// A method call's receiver as written, whitespace collapsed, so the site
    /// tells `common_folders.insert` from `textures.insert`.
    fn receiver_site(&self, expr: &ExprMethodCall) -> String {
        self.snippet(
            self.start_of(expr.receiver.span()),
            self.end_of(expr.receiver.span()),
        )
    }

    /// A `true`/`false` argument or struct field value, flipped. `site` names
    /// what holds it (a field's name, or a callee and the argument's 1-based
    /// position) so a `.cargo/extra_mutants.toml` exclusion matches the flag
    /// without a line number, which drifts.
    fn bool_mutant(&mut self, expr: &Expr, site: &str) {
        let Expr::Lit(ExprLit {
            lit: Lit::Bool(lit),
            ..
        }) = expr
        else {
            return;
        };
        let start = self.start_of(lit.span);
        let end = self.end_of(lit.span);
        let line = self.line_of(start);
        let flipped = if lit.value { "false" } else { "true" };
        let change = format!("{site}: {} becomes {flipped}", lit.value);
        self.push("bool", start, end, flipped, &change, line);
    }
}

impl<'ast> Visit<'ast> for Lister<'_> {
    fn visit_item(&mut self, item: &'ast Item) {
        // #[cfg(test)] and #[test] mark test code, whose mutants nobody wants counted.
        if has_test_attr(item_attrs(item)) {
            return;
        }
        visit::visit_item(self, item);
    }

    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        let outer = std::mem::replace(&mut self.fn_name, item.sig.ident.to_string());
        visit::visit_item_fn(self, item);
        self.fn_name = outer;
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        if has_test_attr(&item.attrs) {
            return;
        }
        let outer = std::mem::replace(&mut self.fn_name, item.sig.ident.to_string());
        visit::visit_impl_item_fn(self, item);
        self.fn_name = outer;
    }

    fn visit_trait_item_fn(&mut self, item: &'ast TraitItemFn) {
        if has_test_attr(&item.attrs) {
            return;
        }
        let outer = std::mem::replace(&mut self.fn_name, item.sig.ident.to_string());
        visit::visit_trait_item_fn(self, item);
        self.fn_name = outer;
    }

    fn visit_item_const(&mut self, _item: &'ast syn::ItemConst) {
        // A const's declared length makes every element removal unviable.
    }

    fn visit_item_static(&mut self, _item: &'ast syn::ItemStatic) {
        // Same as const: a declared length the compiler checks.
    }

    fn visit_expr_if(&mut self, expr: &'ast ExprIf) {
        if expr.else_branch.is_none()
            && !has_let(expr.cond.as_ref())
            && has_early_exit(&expr.then_branch)
        {
            let start = self.start_of(expr.cond.span());
            let end = self.end_of(expr.cond.span());
            let line = self.line_of(start);
            self.push(
                "guard",
                start,
                end,
                "false",
                "the if guard becomes false",
                line,
            );
        }
        visit::visit_expr_if(self, expr);
    }

    fn visit_expr_macro(&mut self, expr: &'ast ExprMacro) {
        if expr.mac.path.is_ident("matches") {
            self.matches_mutants(&expr.mac);
        }
        visit::visit_expr_macro(self, expr);
    }

    fn visit_expr_method_call(&mut self, expr: &'ast ExprMethodCall) {
        let method = expr.method.to_string();
        if NORMALIZE_METHODS.contains(&method.as_str()) && expr.args.is_empty() {
            // `.name()` removed: the dot through the closing parenthesis.
            let start = self.start_of(expr.dot_token.spans[0]);
            let end = self.end_of(expr.paren_token.span.close());
            let line = self.line_of(start);
            let change = format!("the {method}() call is removed");
            self.push("normalize", start, end, "", &change, line);
        }
        if method == "eq_ignore_ascii_case" && expr.args.len() == 1 {
            let start = self.start_of(expr.span());
            let end = self.end_of(expr.span());
            let line = self.line_of(start);
            let receiver = self.snippet(
                self.start_of(expr.receiver.span()),
                self.end_of(expr.receiver.span()),
            );
            let arg = self.snippet(
                self.start_of(expr.args[0].span()),
                self.end_of(expr.args[0].span()),
            );
            self.push(
                "normalize",
                start,
                end,
                format!("{receiver} == {arg}"),
                "eq_ignore_ascii_case becomes ==",
                line,
            );
        }
        let receiver = self.receiver_site(expr);
        for (index, arg) in expr.args.iter().enumerate() {
            self.bool_mutant(arg, &format!("{receiver}.{method} arg {}", index + 1));
        }
        if LIST_METHODS.contains(&method.as_str()) {
            self.array_mutants(&expr.receiver, &method);
            for arg in &expr.args {
                self.array_mutants(arg, &method);
            }
        }
        visit::visit_expr_method_call(self, expr);
    }

    fn visit_expr_call(&mut self, expr: &'ast ExprCall) {
        // The callee as written (`f`, `Foo::new`, a closure's name), as a
        // method call's receiver is.
        let callee = self.snippet(
            self.start_of(expr.func.span()),
            self.end_of(expr.func.span()),
        );
        for (index, arg) in expr.args.iter().enumerate() {
            self.bool_mutant(arg, &format!("{callee} arg {}", index + 1));
        }
        visit::visit_expr_call(self, expr);
    }

    fn visit_expr_struct(&mut self, expr: &'ast ExprStruct) {
        for field in &expr.fields {
            let name = match &field.member {
                syn::Member::Named(ident) => ident.to_string(),
                syn::Member::Unnamed(index) => index.index.to_string(),
            };
            self.bool_mutant(&field.expr, &name);
        }
        visit::visit_expr_struct(self, expr);
    }

    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        if let Stmt::Expr(expr, Some(semicolon)) = stmt
            && let Expr::MethodCall(call) = expr
        {
            let method = call.method.to_string();
            // Any other call made for its effect alone (a repaint asked, a message sent):
            // cargo-mutants never removes a call, so a test that never observes the effect
            // passes either way.
            let kind = if ORDER_METHODS.contains(&method.as_str()) {
                "order"
            } else {
                "call"
            };
            let start = self.start_of(expr.span());
            let end = self.end_of(semicolon.spans[0]);
            let line = self.line_of(start);
            let change = format!("the {method}() statement is deleted");
            self.push(kind, start, end, "", &change, line);
        }
        visit::visit_stmt(self, stmt);
    }
}

/// `matches!(<expr>, <pattern> [if <guard>] [,])`; a `matches!` that does not parse this
/// way is skipped by the caller.
fn matches_args(input: ParseStream) -> syn::Result<Pat> {
    input.parse::<Expr>()?;
    input.parse::<Token![,]>()?;
    let pat = Pat::parse_multi_with_leading_vert(input)?;
    if input.peek(Token![if]) {
        input.parse::<Token![if]>()?;
        input.parse::<Expr>()?;
    }
    if input.peek(Token![,]) {
        input.parse::<Token![,]>()?;
    }
    if !input.is_empty() {
        return Err(input.error("unexpected tokens after the pattern"));
    }
    Ok(pat)
}

/// An `if` whose condition contains `let` (`if let`, a let-chain) is not a plain guard.
fn has_let(cond: &Expr) -> bool {
    struct Finder(bool);
    impl<'ast> Visit<'ast> for Finder {
        fn visit_expr_let(&mut self, _expr: &'ast syn::ExprLet) {
            self.0 = true;
        }
    }
    let mut finder = Finder(false);
    finder.visit_expr(cond);
    finder.0
}

/// Whether a block exits early: `return`, `continue` or `break` anywhere but inside a
/// closure, an `async`/`const` block or a nested item, which have their own control flow.
fn has_early_exit(block: &syn::Block) -> bool {
    struct Finder(bool);
    impl<'ast> Visit<'ast> for Finder {
        fn visit_expr_return(&mut self, _expr: &'ast syn::ExprReturn) {
            self.0 = true;
        }
        fn visit_expr_continue(&mut self, _expr: &'ast syn::ExprContinue) {
            self.0 = true;
        }
        fn visit_expr_break(&mut self, _expr: &'ast syn::ExprBreak) {
            self.0 = true;
        }
        fn visit_expr_closure(&mut self, _expr: &'ast syn::ExprClosure) {}
        fn visit_expr_async(&mut self, _expr: &'ast syn::ExprAsync) {}
        fn visit_expr_const(&mut self, _expr: &'ast syn::ExprConst) {}
        fn visit_item(&mut self, _item: &'ast Item) {}
    }
    let mut finder = Finder(false);
    finder.visit_block(block);
    finder.0
}

/// An element of a literal list: a string, char, byte or integer literal.
fn is_literal(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Lit(ExprLit {
            lit: Lit::Str(_) | Lit::Char(_) | Lit::Byte(_) | Lit::Int(_),
            ..
        })
    )
}

/// The attributes of an item that can hold an expression the lister visits: a
/// body, a discriminant, or a const expression inside a type, a signature or a
/// bound. The rest need no check: `const` and `static` are skipped whole by
/// `visit_item_const`/`visit_item_static`, and `use`, `extern crate`, macro
/// items (a mod-style path, uninterpreted tokens) and verbatim tokens hold no
/// expression at all.
fn item_attrs(item: &Item) -> &[Attribute] {
    match item {
        Item::Enum(item) => &item.attrs,
        Item::Fn(item) => &item.attrs,
        Item::ForeignMod(item) => &item.attrs,
        Item::Impl(item) => &item.attrs,
        Item::Mod(item) => &item.attrs,
        Item::Struct(item) => &item.attrs,
        Item::Trait(item) => &item.attrs,
        Item::TraitAlias(item) => &item.attrs,
        Item::Type(item) => &item.attrs,
        Item::Union(item) => &item.attrs,
        _ => &[],
    }
}

/// `#[test]` or `#[cfg(test)]` (exactly `test`: `cfg(all(test, …))` keeps the code scanned).
pub(crate) fn has_test_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("test") || {
            if !attr.path().is_ident("cfg") {
                return false;
            }
            let mut is_test = false;
            attr.parse_nested_meta(|meta| {
                is_test = meta.path.is_ident("test") && meta.input.is_empty();
                Ok(())
            })
            .is_ok()
                && is_test
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(text: &str) -> Vec<Mutant> {
        scan_source(text).expect("the test sources parse")
    }

    /// The source with a mutant applied, which must still parse.
    fn patch(text: &str, mutant: &Mutant) -> String {
        format!(
            "{}{}{}",
            &text[..mutant.start],
            mutant.replacement,
            &text[mutant.end..]
        )
    }

    fn assert_patches_parse(text: &str, mutants: &[Mutant]) {
        for mutant in mutants {
            let patched = patch(text, mutant);
            syn::parse_file(&patched).unwrap_or_else(|error| {
                panic!("the patched source does not parse: {error}\n{patched}")
            });
        }
    }

    #[test]
    fn drops_a_guard() {
        let text = "fn f(x: bool) -> u32 {\n    if x {\n        return 1;\n    }\n    0\n}\n";
        let mutants = scan(text);
        assert_eq!(mutants.len(), 1);
        let mutant = &mutants[0];
        assert_eq!(mutant.kind, "guard");
        assert_eq!(&text[mutant.start..mutant.end], "x");
        assert_eq!(mutant.replacement, "false");
        assert_eq!(mutant.line, 2);
        assert!(mutant.description.starts_with("in f:"));
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn drops_break_and_continue_guards() {
        let text = "fn f(xs: &[u32]) {\n    for x in xs {\n        if *x == 0 {\n            break;\n        }\n        if *x == 1 {\n            continue;\n        }\n    }\n}\n";
        let mutants = scan(text);
        assert_eq!(mutants.len(), 2);
        assert!(mutants.iter().all(|m| m.kind == "guard"));
        assert_eq!(&text[mutants[0].start..mutants[0].end], "*x == 0");
        assert_eq!(&text[mutants[1].start..mutants[1].end], "*x == 1");
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn removes_matches_alternatives() {
        let text = "fn f(s: &str) -> bool {\n    matches!(s, \"a\" | \"b\" | \"c\")\n}\n";
        let mutants = scan(text);
        assert_eq!(mutants.len(), 3);
        assert!(mutants.iter().all(|m| m.kind == "literal"));
        assert_eq!(&text[mutants[0].start..mutants[0].end], "\"a\" |");
        assert_eq!(&text[mutants[1].start..mutants[1].end], "\"b\" |");
        assert_eq!(&text[mutants[2].start..mutants[2].end], "| \"c\"");
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn removes_matches_alternatives_with_a_guard() {
        let text = "fn f(s: &str) -> bool {\n    matches!(s, \"a\" | \"b\" if s.len() > 1)\n}\n";
        let mutants = scan(text);
        assert_eq!(mutants.len(), 2);
        assert_eq!(&text[mutants[0].start..mutants[0].end], "\"a\" |");
        assert_eq!(&text[mutants[1].start..mutants[1].end], "| \"b\"");
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn removes_array_elements() {
        let text = "fn f(s: &str) -> Vec<&str> {\n    s.split(&[',', ';', '\\t'])\n}\n";
        let mutants = scan(text);
        assert_eq!(mutants.len(), 3);
        assert!(mutants.iter().all(|m| m.kind == "literal"));
        assert_eq!(&text[mutants[0].start..mutants[0].end], "',',");
        assert_eq!(&text[mutants[1].start..mutants[1].end], "';',");
        assert_eq!(&text[mutants[2].start..mutants[2].end], ", '\\t'");
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn removes_array_elements_of_the_receiver() {
        let text = "fn f(c: char) -> bool {\n    [',', ';'].contains(&c)\n}\n";
        let mutants = scan(text);
        assert_eq!(mutants.len(), 2);
        assert_eq!(&text[mutants[0].start..mutants[0].end], "',',");
        assert_eq!(&text[mutants[1].start..mutants[1].end], ", ';'");
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn a_guard_and_its_matches_both_list() {
        let text = "fn f(s: &str) -> bool {\n    if matches!(s, \"a\" | \"b\") {\n        return true;\n    }\n    false\n}\n";
        let mutants = scan(text);
        assert_eq!(mutants.len(), 3);
        assert_eq!(mutants[0].kind, "guard");
        assert_eq!(
            &text[mutants[0].start..mutants[0].end],
            "matches!(s, \"a\" | \"b\")"
        );
        assert!(mutants[1..].iter().all(|m| m.kind == "literal"));
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn an_if_with_else_is_not_a_guard() {
        let text = "fn f(x: bool) -> u32 {\n    if x {\n        return 1;\n    } else {\n        0\n    }\n}\n";
        assert!(scan(text).is_empty());
    }

    #[test]
    fn an_else_if_is_its_own_guard() {
        let text = "fn f(x: bool, y: bool) -> u32 {\n    if x {\n        return 1;\n    } else if y {\n        return 2;\n    }\n    0\n}\n";
        let mutants = scan(text);
        assert_eq!(mutants.len(), 1);
        assert_eq!(&text[mutants[0].start..mutants[0].end], "y");
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn an_if_let_is_not_a_guard() {
        let text = "fn f(x: Option<u32>) -> u32 {\n    if let Some(v) = x {\n        return v;\n    }\n    0\n}\n";
        assert!(scan(text).is_empty());
    }

    #[test]
    fn a_let_chain_is_not_a_guard() {
        let text = "fn f(a: bool, x: Option<u32>) -> u32 {\n    if a && let Some(v) = x {\n        return v;\n    }\n    0\n}\n";
        assert!(scan(text).is_empty());
    }

    #[test]
    fn a_return_in_a_closure_is_not_an_early_exit() {
        let text = "fn f(x: bool) {\n    if x {\n        let g = || {\n            return;\n        };\n    }\n}\n";
        assert!(scan(text).is_empty());
    }

    #[test]
    fn a_return_in_an_async_block_is_not_an_early_exit() {
        let text = "fn f(x: bool) {\n    if x {\n        let g = async {\n            return;\n        };\n    }\n}\n";
        assert!(scan(text).is_empty());
    }

    #[test]
    fn a_return_in_a_nested_item_is_not_an_early_exit() {
        let text = "fn f(x: bool) {\n    if x {\n        fn inner() -> u32 {\n            return 0;\n        }\n    }\n}\n";
        assert!(scan(text).is_empty());
    }

    #[test]
    fn a_cfg_test_module_is_skipped() {
        let text = "fn f(x: bool) {}\n\n#[cfg(test)]\nmod tests {\n    fn g(x: bool) {\n        if x {\n            return;\n        }\n    }\n}\n";
        assert!(scan(text).is_empty());
    }

    #[test]
    fn a_test_fn_is_skipped() {
        let text = "#[test]\nfn t() {\n    let x = false;\n    if x {\n        return;\n    }\n}\n";
        assert!(scan(text).is_empty());
    }

    #[test]
    fn a_single_alternative_matches_gives_nothing() {
        let text = "fn f(s: &str) -> bool {\n    matches!(s, \"a\")\n}\n";
        assert!(scan(text).is_empty());
    }

    #[test]
    fn a_single_element_array_gives_nothing() {
        let text = "fn f(s: &str) -> Vec<&str> {\n    s.split(&[','])\n}\n";
        assert!(scan(text).is_empty());
    }

    #[test]
    fn an_unlisted_method_gives_nothing() {
        let text = "fn f(s: &str) -> bool {\n    s.chars().eq(['a', 'b'])\n}\n";
        assert!(scan(text).is_empty());
    }

    #[test]
    fn removes_a_normalize_call() {
        let text = "fn f(s: &str) -> &str {\n    s.trim()\n}\n";
        let mutants = scan(text);
        assert_eq!(mutants.len(), 1);
        let mutant = &mutants[0];
        assert_eq!(mutant.kind, "normalize");
        assert_eq!(&text[mutant.start..mutant.end], ".trim()");
        assert_eq!(mutant.replacement, "");
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn eq_ignore_ascii_case_becomes_eq() {
        let text = "fn f(a: &str, b: &str) -> bool {\n    a.eq_ignore_ascii_case(b)\n}\n";
        let mutants = scan(text);
        assert_eq!(mutants.len(), 1);
        let mutant = &mutants[0];
        assert_eq!(mutant.kind, "normalize");
        assert_eq!(&text[mutant.start..mutant.end], "a.eq_ignore_ascii_case(b)");
        assert_eq!(mutant.replacement, "a == b");
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn a_normalize_method_with_arguments_gives_nothing() {
        let text = "fn f(s: &str) -> &str {\n    s.trim_end(' ')\n}\n";
        assert!(scan(text).is_empty());
    }

    #[test]
    fn deletes_an_order_statement() {
        let text = "fn f(v: &mut Vec<u32>) {\n    v.sort();\n    v.reverse();\n}\n";
        let mutants = scan(text);
        assert_eq!(mutants.len(), 2);
        assert!(mutants.iter().all(|m| m.kind == "order"));
        assert_eq!(&text[mutants[0].start..mutants[0].end], "v.sort();");
        assert_eq!(&text[mutants[1].start..mutants[1].end], "v.reverse();");
        assert_eq!(mutants[0].replacement, "");
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn deletes_any_other_method_call_statement_as_a_call_mutant() {
        let text = "fn f(ctx: &Ctx, v: &mut Vec<u32>) {\n    ctx.request_repaint();\n    v.sort();\n    let _id = ctx.id();\n}\n";
        let mutants = scan(text);
        let kinds: Vec<&str> = mutants.iter().map(|m| m.kind).collect();
        assert_eq!(
            kinds,
            ["call", "order"],
            "one mutant a statement, none for the let"
        );
        assert_eq!(
            &text[mutants[0].start..mutants[0].end],
            "ctx.request_repaint();"
        );
        assert_eq!(mutants[0].replacement, "");
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn an_order_method_as_an_expression_gives_nothing() {
        let text = "fn f(v: &mut Vec<u32>) {\n    let _x = v.sort();\n}\n";
        assert!(scan(text).is_empty());
        let text = "fn f(v: &mut Vec<u32>) {\n    v.sort()\n}\n";
        assert!(scan(text).is_empty());
    }

    #[test]
    fn flips_a_bool_argument() {
        let text = "fn f(x: bool) {}\nfn g() {\n    f(true);\n}\n";
        let mutants = scan(text);
        assert_eq!(mutants.len(), 1);
        let mutant = &mutants[0];
        assert_eq!(mutant.kind, "bool");
        assert_eq!(&text[mutant.start..mutant.end], "true");
        assert_eq!(mutant.replacement, "false");
        assert_eq!(
            mutant.description,
            "in g: f arg 1: true becomes false | removed: true"
        );
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn flips_a_bool_method_argument() {
        let text =
            "fn f(s: &str) -> bool {\n    s.starts_with(\"x\") && s.contains(\"y\", true)\n}\n";
        let mutants = scan(text);
        let mutant = mutants
            .iter()
            .find(|m| m.kind == "bool")
            .expect("a bool mutant");
        assert_eq!(&text[mutant.start..mutant.end], "true");
        assert_eq!(mutant.replacement, "false");
        assert_eq!(
            mutant.description,
            "in f: s.contains arg 2: true becomes false | removed: true"
        );
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn a_method_arg_names_its_receiver() {
        // Two calls of one method on different receivers get different
        // descriptions, so an exclusion names the flag it means.
        let text = "fn f() {\n    textures.insert(true, &below);\n    common_folders.insert(true, stem);\n}\n";
        // Each statement also gives a `call` mutant, not this test's subject.
        let mutants: Vec<Mutant> = scan(text)
            .into_iter()
            .filter(|m| m.kind == "bool")
            .collect();
        assert_eq!(mutants.len(), 2);
        assert_eq!(
            mutants[0].description,
            "in f: textures.insert arg 1: true becomes false | removed: true"
        );
        assert_eq!(
            mutants[1].description,
            "in f: common_folders.insert arg 1: true becomes false | removed: true"
        );
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn flips_a_struct_field_bool() {
        let text = "struct S {\n    flag: bool,\n}\nfn f() -> S {\n    S { flag: true }\n}\n";
        let mutants = scan(text);
        assert_eq!(mutants.len(), 1);
        let mutant = &mutants[0];
        assert_eq!(mutant.kind, "bool");
        assert_eq!(&text[mutant.start..mutant.end], "true");
        assert_eq!(mutant.replacement, "false");
        assert_eq!(
            mutant.description,
            "in f: flag: true becomes false | removed: true"
        );
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn a_bool_in_a_const_gives_nothing() {
        let text = "const F: bool = true;\nfn f() {}\n";
        assert!(scan(text).is_empty());
    }

    #[test]
    fn a_bool_in_a_match_arm_gives_nothing() {
        let text =
            "fn f(x: bool) {\n    match x {\n        true => {}\n        false => {}\n    }\n}\n";
        assert!(scan(text).is_empty());
    }

    #[test]
    fn a_bool_in_a_comparison_gives_nothing() {
        let text = "fn f(x: bool) -> bool {\n    x == true\n}\n";
        assert!(scan(text).is_empty());
    }

    #[test]
    fn a_bool_in_matches_gives_nothing() {
        let text = "fn f(x: bool) -> bool {\n    matches!(x, true)\n}\n";
        assert!(scan(text).is_empty());
    }

    #[test]
    fn byte_offsets_ignore_a_multibyte_char_on_the_line() {
        // caf\u{e9}: the \u{e9} is two bytes, one char; a column counted in bytes would
        // land the mutant one byte early.
        let text = "fn f(x: bool) {\n    if /* caf\u{e9} */ x {\n        return;\n    }\n}\n";
        let mutants = scan(text);
        assert_eq!(&text[mutants[0].start..mutants[0].end], "x");
        assert_eq!(mutants[0].line, 2);
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn the_second_file_in_a_process_gets_its_own_offsets() {
        // A first parse fills the proc-macro2 source map; the second file's spans must
        // still map into its own text.
        let first = "fn a(long_condition_name: bool) {\n    if long_condition_name {\n        return;\n    }\n}\n";
        scan(first);
        let text = "fn f(x: bool) {\n    if x {\n        return;\n    }\n}\n";
        let mutants = scan(text);
        assert_eq!(mutants.len(), 1);
        assert_eq!(&text[mutants[0].start..mutants[0].end], "x");
        assert_eq!(mutants[0].line, 2);
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn a_guard_in_an_impl_method_is_listed() {
        let text = "struct S;\nimpl S {\n    #[cfg(test)]\n    fn skipped(&self, x: u32) {\n        if x > 9 {\n            return;\n        }\n    }\n    fn m(&self, x: u32) {\n        if x > 1 {\n            return;\n        }\n    }\n}\n";
        let mutants = scan(text);
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].kind, "guard");
        assert_eq!(&text[mutants[0].start..mutants[0].end], "x > 1");
        assert!(mutants[0].description.starts_with("in m:"));
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn a_guard_in_a_trait_default_method_is_listed() {
        let text = "trait T {\n    #[cfg(test)]\n    fn skipped(&self, x: u32) {\n        if x > 9 {\n            return;\n        }\n    }\n    fn m(&self, x: u32) {\n        if x > 1 {\n            return;\n        }\n    }\n}\n";
        let mutants = scan(text);
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].kind, "guard");
        assert_eq!(&text[mutants[0].start..mutants[0].end], "x > 1");
        assert!(mutants[0].description.starts_with("in m:"));
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn a_cfg_test_impl_is_skipped() {
        // The const's initializer is visited code too (`visit_impl_item_const`
        // is not overridden): only the `#[cfg(test)]` on the impl skips it all.
        let text = "#[cfg(test)]\nimpl S {\n    const C: usize = f(true);\n    fn m(&self, x: u32) {\n        if x > 1 {\n            return;\n        }\n    }\n}\n";
        assert!(scan(text).is_empty());
    }

    #[test]
    fn a_cfg_test_trait_is_skipped() {
        let text = "#[cfg(test)]\ntrait T {\n    fn m(&self, x: u32) {\n        if x > 1 {\n            return;\n        }\n    }\n}\n";
        assert!(scan(text).is_empty());
    }

    #[test]
    fn other_cfg_attributes_do_not_mark_test_code() {
        let text = "#[cfg(windows)]\nfn f(x: u32) {\n    if x > 1 {\n        return;\n    }\n}\n\n#[cfg(feature = \"x\")]\nfn g(x: u32) {\n    if x > 2 {\n        return;\n    }\n}\n";
        let mutants = scan(text);
        assert_eq!(mutants.len(), 2);
        assert_eq!(&text[mutants[0].start..mutants[0].end], "x > 1");
        assert_eq!(&text[mutants[1].start..mutants[1].end], "x > 2");
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn a_mixed_list_removes_only_literal_elements() {
        let text = "impl S {\n    fn m(&self, s: &str) -> Vec<&str> {\n        s.split(&[\",\", Self::SEP, \";\"])\n    }\n}\n";
        let mutants = scan(text);
        assert_eq!(mutants.len(), 2);
        assert!(mutants.iter().all(|m| m.kind == "literal"));
        assert_eq!(&text[mutants[0].start..mutants[0].end], "\",\",");
        assert_eq!(&text[mutants[1].start..mutants[1].end], ", \";\"");
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn an_inner_attribute_does_not_shift_the_offsets() {
        // `#![...]` is an inner attribute, not a shebang: nothing is stripped.
        let text =
            "#![allow(dead_code)]\nfn f(x: u32) {\n    if x > 1 {\n        return;\n    }\n}\n";
        let mutants = scan(text);
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].line, 3);
        assert_eq!(&text[mutants[0].start..mutants[0].end], "x > 1");
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn a_bom_does_not_shift_the_offsets() {
        // The byte range indexes the original text, BOM included.
        let text = "\u{feff}fn f(x: u32) {\n    if x > 1 {\n        return;\n    }\n}\n";
        let mutants = scan(text);
        assert_eq!(mutants.len(), 1);
        assert_eq!(&text[mutants[0].start..mutants[0].end], "x > 1");
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn a_shebang_does_not_shift_the_offsets() {
        // The shebang line, newline included, is stripped; the byte range still
        // indexes the original text.
        let text =
            "#!/usr/bin/env bash\nfn f(x: u32) {\n    if x > 1 {\n        return;\n    }\n}\n";
        let mutants = scan(text);
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].line, 2);
        assert_eq!(&text[mutants[0].start..mutants[0].end], "x > 1");
        assert_patches_parse(text, &mutants);
    }

    #[test]
    fn a_cfg_test_item_holding_an_expression_is_skipped() {
        // These arms of `item_attrs` stay: a discriminant, an array length or a
        // const generic argument is an expression the lister visits, so the
        // `#[cfg(test)]` check must see the item's attributes.
        let text = "#[cfg(test)]\nenum E {\n    A = f(true),\n}\n\n#[cfg(test)]\nstruct S {\n    x: [u8; f(true)],\n}\n\n#[cfg(test)]\nunion U {\n    x: [u8; f(true)],\n}\n\n#[cfg(test)]\ntype T = [u8; f(true)];\n\n#[cfg(test)]\ntrait A = B<{ f(true) }>;\n\n#[cfg(test)]\nextern \"C\" {\n    static X: [u8; f(true)];\n}\n";
        assert!(scan(text).is_empty());
    }
}
