//! The tree a rule reads of a file (docs/FIX.md, "The tree"): every node of the parser's trees, named and laid
//! out as scalameta names and lays out its own, each with its span, its parent and its children by field. It is
//! made in one walk of the file's ASTs from what the parser recorded under `Syntax::keep` and `Syntax::index`
//! (`SyntaxNotes`, `Ast::name_spans`, `import_names`, `def_ranges`); a name the parser keeps no span of is the
//! lexer's token where its node starts (`SourceKept::tokens`). What the parser made up (a case class's companion,
//! a `derives` clause's given, a pattern definition's vals, a `@main` method's proxy) is no node.

use crate::ast::*;
use crate::intern::{Interner, Name};
use crate::source::Span;
use crate::token::{Tok, Token};

pub const NONE: u32 = u32::MAX;

macro_rules! names {
    ($ty:ident, $table:ident: $($v:ident = $s:literal),* $(,)?) => {
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        #[repr(u16)]
        pub enum $ty { $($v),* }
        pub const $table: &[&str] = &[$($s),*];
        impl $ty {
            #[allow(dead_code)]
            pub fn name(self) -> &'static str { $table[self as usize] }
        }
    };
}

names!(K, KIND_NAMES:
    Source = "Source", Pkg = "Pkg", Import = "Import", Export = "Export", Importer = "Importer",
    ImporteeName = "Importee.Name", ImporteeRename = "Importee.Rename", ImporteeUnimport = "Importee.Unimport",
    ImporteeWildcard = "Importee.Wildcard", ImporteeGiven = "Importee.Given", ImporteeGivenAll = "Importee.GivenAll",
    DefnVal = "Defn.Val", DefnVar = "Defn.Var", DeclVal = "Decl.Val", DeclVar = "Decl.Var", DefnDef = "Defn.Def",
    DeclDef = "Decl.Def", DefnClass = "Defn.Class", DefnTrait = "Defn.Trait", DefnObject = "Defn.Object",
    DefnEnum = "Defn.Enum", DefnEnumCase = "Defn.EnumCase", DefnGiven = "Defn.Given", DefnGivenAlias = "Defn.GivenAlias",
    DefnType = "Defn.Type", DeclType = "Decl.Type",
    Template = "Template", Init = "Init", ParamClause = "Term.ParamClause", Param = "Term.Param",
    TypeParam = "Type.Param",
    ModAnnot = "Mod.Annot", ModPrivate = "Mod.Private", ModProtected = "Mod.Protected", ModSealed = "Mod.Sealed",
    ModAbstract = "Mod.Abstract", ModFinal = "Mod.Final", ModCase = "Mod.Case", ModLazy = "Mod.Lazy",
    ModOverride = "Mod.Override", ModOpaque = "Mod.Opaque", ModInline = "Mod.Inline", ModOpen = "Mod.Open",
    ModInfix = "Mod.Infix", ModImplicit = "Mod.Implicit", ModTransparent = "Mod.Transparent",
    ModCovariant = "Mod.Covariant", ModContravariant = "Mod.Contravariant",
    TermName = "Term.Name", TypeName = "Type.Name", NamePlaceholder = "Name.Placeholder", TermPlaceholder = "Term.Placeholder",
    TermThis = "Term.This", TermSuper = "Term.Super", TermSelect = "Term.Select", TermApply = "Term.Apply",
    TermApplyUsing = "Term.ApplyUsing", TermApplyType = "Term.ApplyType", TermApplyInfix = "Term.ApplyInfix",
    TermApplyUnary = "Term.ApplyUnary", TermAssign = "Term.Assign", TermBlock = "Term.Block", TermIf = "Term.If",
    TermMatch = "Term.Match", TermTry = "Term.Try", TermWhile = "Term.While", TermFor = "Term.For",
    TermForYield = "Term.ForYield", TermFunction = "Term.Function", TermAnonymousFunction = "Term.AnonymousFunction",
    TermPolyFunction = "Term.PolyFunction",
    TermReturn = "Term.Return", TermThrow = "Term.Throw", TermNew = "Term.New", TermNewAnonymous = "Term.NewAnonymous",
    TermInterpolate = "Term.Interpolate", TermTuple = "Term.Tuple", TermAscribe = "Term.Ascribe",
    TermParens = "Term.Parens", TermQuote = "Term.QuotedMacroExpr", TermSplice = "Term.SplicedMacroExpr",
    TermEta = "Term.Eta", TermRepeated = "Term.Repeated",
    LitInt = "Lit.Int", LitLong = "Lit.Long", LitDouble = "Lit.Double", LitFloat = "Lit.Float",
    LitBoolean = "Lit.Boolean", LitChar = "Lit.Char", LitString = "Lit.String", LitUnit = "Lit.Unit", LitNull = "Lit.Null",
    Case = "Case", EnumGenerator = "Enumerator.Generator", EnumCaseGenerator = "Enumerator.CaseGenerator",
    EnumGuard = "Enumerator.Guard", EnumVal = "Enumerator.Val",
    PatWildcard = "Pat.Wildcard", PatVar = "Pat.Var", PatBind = "Pat.Bind", PatTyped = "Pat.Typed",
    PatExtract = "Pat.Extract", PatExtractInfix = "Pat.ExtractInfix", PatTuple = "Pat.Tuple",
    PatAlternative = "Pat.Alternative", PatSeqWildcard = "Pat.SeqWildcard", PatInterpolate = "Pat.Interpolate",
    TypeSelect = "Type.Select", TypeApply = "Type.Apply", TypeFunction = "Type.Function",
    TypeContextFunction = "Type.ContextFunction", TypeTuple = "Type.Tuple", TypeOr = "Type.Or", TypeAnd = "Type.And",
    TypeByName = "Type.ByName", TypeRepeated = "Type.Repeated", TypeWildcard = "Type.Wildcard",
    TypeLambda = "Type.Lambda", TypePolyFunction = "Type.PolyFunction", TypeSingleton = "Type.Singleton",
    TypeProject = "Type.Project", TypeRefine = "Type.Refine", TypeAnnotate = "Type.Annotate", TypeMatch = "Type.Match",
    TypeCase = "TypeCase", TypeVar = "Type.Var",
    Other = "Other",
);

names!(F, FIELD_NAMES:
    Stats = "stats", Ref = "ref", Importers = "importers", Importees = "importees", Name = "name", Rename = "rename",
    Tpe = "tpe", Mods = "mods", Pats = "pats", Decltpe = "decltpe", Rhs = "rhs", Tparams = "tparams",
    Paramss = "paramss", Body = "body", Templ = "templ", Ctor = "ctor", Inits = "inits", SelfDecl = "self",
    Argss = "argss", Values = "values", Default = "default", Init = "init", Qual = "qual", Fun = "fun", Args = "args",
    Lhs = "lhs", Op = "op", Targs = "targs", Arg = "arg", Cond = "cond", Thenp = "thenp", Elsep = "elsep",
    Expr = "expr", Cases = "cases", Catchp = "catchp", Finallyp = "finallyp", Enums = "enums", Params = "params",
    Prefix = "prefix", Pat = "pat", Lo = "lo", Hi = "hi", Annots = "annots",
    Other = "children",
);

/// A node: its kind, its span, its parent, its children (`Tree::entries[first..first + count]`, each with its
/// field, in source order), its text where it has one (a name's, a literal's value as written) and its flags.
#[derive(Clone, Copy, Debug)]
pub struct Node {
    pub kind: K,
    pub span: Span,
    pub parent: u32,
    pub first: u32,
    pub count: u32,
    pub value: u32,
    pub flags: u32,
}

/// Flags of a node.
pub mod flags {
    /// A parameter clause or a parameter of one written `using`.
    pub const USING: u32 = 1;
    /// A parameter clause or a parameter of one written `implicit`.
    pub const IMPLICIT: u32 = 2;
    /// A `val` or `var` parameter of a class.
    pub const VAL: u32 = 4;
    pub const VAR: u32 = 8;
    /// A lambda's parameter written without its type.
    pub const UNTYPED: u32 = 16;
}

#[derive(Default)]
pub struct Tree {
    pub nodes: Vec<Node>,
    pub entries: Vec<(F, u32)>,
    pub values: Vec<String>,
}

impl Tree {
    /// The tree of a script's own text, `[0, limit)` (`teq fix` edits a `.sc` as written; `script::wrap` appends the
    /// wrapper past it for typing alone): the nodes inside it, those whose parent is not (the script's statements,
    /// which the parser moves into the wrapper's class) a `Source`'s over the text.
    pub fn within(&self, limit: u32) -> Tree {
        let mut out = Tree::default();
        let mut map = vec![NONE; self.nodes.len()];
        let root = self.nodes.len().saturating_sub(1);
        // Children before their parents, as `Builder::make` lays them out.
        for i in 0..root {
            let n = &self.nodes[i];
            if n.span.end > limit {
                continue;
            }
            let kids: Vec<(F, u32)> = self.children(i as u32).iter().filter(|&&(_, c)| map[c as usize] != NONE).map(|&(f, c)| (f, map[c as usize])).collect();
            let id = out.nodes.len() as u32;
            for &(_, c) in &kids {
                out.nodes[c as usize].parent = id;
            }
            let value = match self.value(i as u32) {
                Some(v) => {
                    out.values.push(v.to_string());
                    (out.values.len() - 1) as u32
                }
                None => NONE,
            };
            let first = out.entries.len() as u32;
            out.entries.extend(kids.iter().copied());
            out.nodes.push(Node { kind: n.kind, span: n.span, parent: NONE, first, count: kids.len() as u32, value, flags: n.flags });
            map[i] = id;
        }
        let mut stats: Vec<u32> = (0..out.nodes.len() as u32).filter(|&n| out.nodes[n as usize].parent == NONE).collect();
        stats.sort_by_key(|&n| out.nodes[n as usize].span.start);
        let id = out.nodes.len() as u32;
        for &s in &stats {
            out.nodes[s as usize].parent = id;
        }
        let first = out.entries.len() as u32;
        out.entries.extend(stats.iter().map(|&s| (F::Stats, s)));
        out.nodes.push(Node { kind: K::Source, span: Span::new(0, limit), parent: NONE, first, count: stats.len() as u32, value: NONE, flags: 0 });
        out
    }

    pub fn children(&self, n: u32) -> &[(F, u32)] {
        let node = &self.nodes[n as usize];
        &self.entries[node.first as usize..(node.first + node.count) as usize]
    }

    pub fn value(&self, n: u32) -> Option<&str> {
        let v = self.nodes[n as usize].value;
        (v != NONE).then(|| self.values[v as usize].as_str())
    }

}

impl Tree {
    /// The tree as text, a node a line: its kind and its text, its children indented (`TEQ_FIX_TREE`).
    pub fn show(&self, text: &str) -> String {
        let mut out = String::new();
        let mut stack = vec![(self.nodes.len() as u32 - 1, 0usize)];
        while let Some((n, depth)) = stack.pop() {
            let node = self.nodes[n as usize];
            let s = &text[node.span.start as usize..node.span.end as usize];
            let shown: String = s.chars().take(60).collect::<String>().replace('\n', "⏎");
            out.push_str(&format!("{}{} `{}`\n", "  ".repeat(depth), node.kind.name(), shown));
            for &(_, c) in self.children(n).iter().rev() {
                stack.push((c, depth + 1));
            }
        }
        out
    }
}

/// The tree of a file from its ASTs (the file's, then its package blocks', `frontend::lay_out`), the root its last
/// node.
pub fn build(text: &str, asts: &[&Ast], kept: &crate::ast::SourceKept, interner: &Interner) -> Tree {
    let mut b = Builder { text, interner, tokens: &kept.tokens, out: Tree::default(), ast: asts[0], patdefs: Vec::new() };
    let mut top: Vec<(F, u32)> = Vec::new();
    let outer_imports: Vec<Span> = asts[0].imports.iter().map(|i| i.span).collect();
    for (k, &ast) in asts.iter().enumerate() {
        b.ast = ast;
        let mut stats: Vec<(F, u32)> = Vec::new();
        let own_imports: Vec<&Import> = ast.imports.iter().chain(&ast.language_imports).filter(|i| k == 0 || !outer_imports.contains(&i.span)).collect();
        for stmt in b.import_statements(&own_imports, &ast.import_ranges, K::Import) {
            stats.push((F::Stats, stmt));
        }
        for stmt in b.import_statements(&ast.top_exports.iter().collect::<Vec<_>>(), &[], K::Export) {
            stats.push((F::Stats, stmt));
        }
        for d in b.defs_written(&ast.top_level) {
            stats.push((F::Stats, d));
        }
        let packages = b.packages(&ast.package_ranges);
        let mut nested = stats;
        for (range, names) in packages.into_iter().rev() {
            let span = Span::new(range.start, nested.iter().map(|&(_, n)| b.out.nodes[n as usize].span.end).max().unwrap_or(range.end).max(range.end));
            let mut fields = vec![(F::Ref, names)];
            fields.extend(nested);
            let pkg = b.make(K::Pkg, span, fields, None, 0);
            nested = vec![(F::Stats, pkg)];
        }
        top.extend(nested);
    }
    b.make(K::Source, Span::new(0, text.len() as u32), top, None, 0);
    // A number literal's span leaves its suffix out (`1L`, `1.5f`: `Lexer::number`); a node's span is its source span,
    // so a node that ends where a suffixed literal's digits end, the literal and its ancestors, ends after the suffix.
    let mut ends: Vec<u32> = kept.tokens.iter().filter(|t| super::source::suffixed(text, t.kind, t.span.end)).map(|t| t.span.end).collect();
    if !ends.is_empty() {
        ends.sort_unstable();
        ends.dedup();
        for n in &mut b.out.nodes {
            if ends.binary_search(&n.span.end).is_ok() {
                n.span.end += 1;
            }
        }
    }
    b.out
}

struct Builder<'a> {
    text: &'a str,
    interner: &'a Interner,
    tokens: &'a [Token],
    out: Tree,
    ast: &'a Ast,
    /// The ranges of the pattern definitions made a node: their holder and their vals share it.
    patdefs: Vec<Span>,
}

impl<'a> Builder<'a> {
    /// A node over `fields`, each child's parent set; the fields in source order.
    fn make(&mut self, kind: K, span: Span, mut fields: Vec<(F, u32)>, value: Option<String>, flags: u32) -> u32 {
        fields.retain(|&(_, c)| c != NONE);
        fields.sort_by_key(|&(_, c)| self.out.nodes[c as usize].span.start);
        let id = self.out.nodes.len() as u32;
        for &(_, c) in &fields {
            self.out.nodes[c as usize].parent = id;
        }
        let value = match value {
            Some(v) => {
                self.out.values.push(v);
                (self.out.values.len() - 1) as u32
            }
            None => NONE,
        };
        let first = self.out.entries.len() as u32;
        let count = fields.len() as u32;
        self.out.entries.extend(fields);
        self.out.nodes.push(Node { kind, span, parent: NONE, first, count, value, flags });
        id
    }

    fn leaf(&mut self, kind: K, span: Span, value: Option<String>) -> u32 {
        self.make(kind, span, Vec::new(), value, 0)
    }

    fn name_text(&self, n: Name) -> String {
        self.interner.get(n).to_string()
    }

    fn slice(&self, span: Span) -> &'a str {
        &self.text[span.start as usize..span.end as usize]
    }

    /// The lexer's identifier (or keyword, or `_`) where `at` starts: the name of a node the parser kept no span of.
    fn token_at(&self, at: u32) -> Option<Span> {
        let i = self.tokens.partition_point(|t| t.span.start < at);
        self.tokens.get(i).filter(|t| t.span.start == at && t.span.end > t.span.start).map(|t| t.span)
    }

    /// A name's node at `span`, of `kind`, its text the interned name's.
    fn name(&mut self, kind: K, span: Span, n: Name) -> u32 {
        if span.end <= span.start {
            return NONE;
        }
        // A placeholder's name is a fresh one (`_$1`); what was written is `_`.
        let written = self.slice(span);
        let (kind, text) = if written == "_" && kind == K::TermName { (K::NamePlaceholder, "_".to_string()) } else { (kind, self.name_text(n)) };
        self.leaf(kind, span, Some(text))
    }

    /// The name that starts at `at`, by the lexer's token there.
    fn name_from(&mut self, kind: K, at: u32, n: Name) -> u32 {
        match self.token_at(at) {
            Some(span) => self.name(kind, span, n),
            None => NONE,
        }
    }

    // ---- packages and imports ----

    /// The package clauses written in this AST, each with its range and its path's node.
    fn packages(&mut self, ranges: &[Span]) -> Vec<(Span, u32)> {
        let names: Vec<Span> = self.ast.notes.package_names.clone();
        ranges
            .iter()
            .map(|&r| {
                let parts: Vec<Span> = names.iter().copied().filter(|s| s.start >= r.start && s.end <= r.end).collect();
                let path = self.path(&parts, K::TermName);
                (r, path)
            })
            .collect()
    }

    /// `a.b.c` from the spans of its names: a `Term.Name` and a `Term.Select` per name after it.
    fn path(&mut self, parts: &[Span], last: K) -> u32 {
        let mut node = NONE;
        for (i, &s) in parts.iter().enumerate() {
            let text = self.slice(s).trim_matches('`').to_string();
            let kind = if i + 1 == parts.len() { last } else { K::TermName };
            let name = self.leaf(kind, s, Some(text));
            node = if node == NONE {
                name
            } else {
                let start = self.out.nodes[node as usize].span.start;
                let k = if kind == K::TypeName { K::TypeSelect } else { K::TermSelect };
                self.make(k, Span::new(start, s.end), vec![(F::Qual, node), (F::Name, name)], None, 0)
            };
        }
        node
    }

    /// The statements of selectors `sels` (one entry per selector), grouped by statement: a top-level statement by
    /// `ranges` (`Ast::import_ranges`), a local one by `Ast::import_stmts`.
    fn import_statements(&mut self, sels: &[&Import], ranges: &[Span], kind: K) -> Vec<u32> {
        let mut groups: Vec<Vec<&Import>> = Vec::new();
        for &s in sels {
            let start_new = match groups.last() {
                None => true,
                Some(g) => {
                    let first = g[0];
                    match ranges.iter().find(|r| r.start <= first.tree_start && first.tree_start < r.end) {
                        Some(r) => !(r.start <= s.tree_start && s.tree_start < r.end),
                        None => first.tree_start != s.tree_start && !self.continues_statement(g, s),
                    }
                }
            };
            if start_new {
                groups.push(vec![s]);
            } else {
                groups.last_mut().unwrap().push(s);
            }
        }
        groups.into_iter().map(|g| self.import_statement(&g, kind)).collect()
    }

    /// Whether `s` is a later clause of the statement `g` holds (`import a.b, c.d`): it starts after a comma.
    fn continues_statement(&self, g: &[&Import], s: &Import) -> bool {
        let last = g.last().unwrap();
        let between = &self.text[last.clause_end.min(s.tree_start) as usize..s.tree_start as usize];
        between.trim_start().starts_with(',')
    }

    fn import_statement(&mut self, sels: &[&Import], kind: K) -> u32 {
        let mut importers: Vec<(F, u32)> = Vec::new();
        let mut i = 0;
        while i < sels.len() {
            let start = sels[i].tree_start;
            let j = sels[i..].iter().position(|s| s.tree_start != start).map_or(sels.len(), |k| i + k);
            importers.push((F::Importers, self.importer(&sels[i..j])));
            i = j;
        }
        let first = sels[0].tree_start;
        // A statement's first clause starts at its keyword (`Import::tree_start`); its end is its last clause's.
        let end = sels.last().unwrap().clause_end;
        self.make(kind, Span::new(first, end), importers, None, 0)
    }

    fn importer(&mut self, sels: &[&Import]) -> u32 {
        let first = sels[0];
        let names = self.import_names(first.span);
        let path_len = first.path.len().min(names.len());
        let path = self.path(&names[..path_len], K::TermName);
        let mut fields = vec![(F::Ref, path)];
        for s in sels {
            fields.push((F::Importees, self.importee(s)));
        }
        let start = self.out.nodes.get(path as usize).map_or(first.span.start, |n| n.span.start);
        let span = Span::new(start, first.clause_end.max(sels.last().unwrap().selector_span.end));
        self.make(K::Importer, span, fields, None, 0)
    }

    fn import_names(&self, span: Span) -> Vec<Span> {
        self.ast.import_names.iter().find(|(s, _)| *s == span).map(|(_, v)| v.clone()).unwrap_or_default()
    }

    fn importee(&mut self, s: &Import) -> u32 {
        let names = self.import_names(s.span);
        let sel_names: Vec<Span> = names.get(s.path.len()..).map(<[Span]>::to_vec).unwrap_or_default();
        match s.sel {
            ImportSel::Wildcard => self.leaf(K::ImporteeWildcard, s.selector_span, None),
            ImportSel::Given => match s.bound {
                Some(t) => {
                    let tpe = self.ty(t);
                    self.make(K::ImporteeGiven, s.selector_span, vec![(F::Tpe, tpe)], None, 0)
                }
                None => self.leaf(K::ImporteeGivenAll, s.selector_span, None),
            },
            ImportSel::Name(n, rename) => {
                let name_span = sel_names.first().copied().unwrap_or(s.selector_span);
                let name = self.name(K::TermName, name_span, n);
                match rename {
                    None => self.make(K::ImporteeName, s.selector_span, vec![(F::Name, name)], None, 0),
                    Some(r) if self.interner.get(r) == "_" => self.make(K::ImporteeUnimport, s.selector_span, vec![(F::Name, name)], None, 0),
                    Some(r) => {
                        let rename = match sel_names.get(1) {
                            Some(&rs) => self.name(K::TermName, rs, r),
                            None => NONE,
                        };
                        self.make(K::ImporteeRename, s.selector_span, vec![(F::Name, name), (F::Rename, rename)], None, 0)
                    }
                }
            }
        }
    }

    // ---- definitions ----

    /// The written definitions of `defs`: what the parser made up left out, the names of one `val a, b = e` one
    /// definition with their patterns.
    fn defs_written(&mut self, defs: &[DefId]) -> Vec<u32> {
        let mut out = Vec::new();
        for &d in defs {
            if let Some(n) = self.def(d) {
                out.push(n);
            }
        }
        out
    }

    fn made_up(&self, d: DefId) -> bool {
        let ast = self.ast;
        let range = ast.def_range(d);
        range == crate::ast::NO_RANGE
            || ast.synthetic_companions.contains(&d)
            || ast.derived_givens.contains(&d)
            || ast.main_proxies.iter().any(|&(p, _)| p == d)
            || ast.val_copies.contains_key(&d)
    }

    /// The pattern definition the parser lowered into the holder or component `d` (`parse_val`'s `makePatDef`): the
    /// definition as written, whose pattern and right side it keeps, once for the range they share.
    fn pattern_def(&mut self, d: DefId) -> Option<u32> {
        let ast: &'a Ast = self.ast;
        let range = ast.def_range(d);
        if self.patdefs.contains(&range) {
            return None;
        }
        self.patdefs.push(range);
        let original = (0..ast.defs.len()).map(|i| DefId(i as u32)).find(|&o| matches!(ast.def(o).kind, DefKind::Val { pat: Some(_), .. }) && ast.def_range(o) == range)?;
        let def: &'a Def = ast.def(original);
        let DefKind::Val { pat: Some(p), ty, rhs } = def.kind else { return None };
        let mut fields = self.mods(original, def);
        fields.push((F::Pats, self.pat(p)));
        if let Some(t) = ty {
            fields.push((F::Decltpe, self.ty(t)));
        }
        if let Some(r) = rhs {
            fields.push((F::Rhs, self.expr(r)));
        }
        let kind = if def.mods & mods::MUTABLE != 0 { K::DefnVar } else { K::DefnVal };
        Some(self.make(kind, range, fields, None, 0))
    }

    fn mods(&mut self, d: DefId, def: &Def) -> Vec<(F, u32)> {
        let mut out: Vec<(F, u32)> = Vec::new();
        for a in &def.annots {
            out.push((F::Mods, self.annot(a)));
        }
        for (bit, span) in self.written_modifiers(d, def) {
            let kind = match bit {
                mods::PRIVATE => K::ModPrivate,
                mods::PROTECTED => K::ModProtected,
                mods::SEALED => K::ModSealed,
                mods::ABSTRACT => K::ModAbstract,
                mods::FINAL => K::ModFinal,
                mods::CASE => K::ModCase,
                mods::LAZY => K::ModLazy,
                mods::OVERRIDE => K::ModOverride,
                mods::OPAQUE => K::ModOpaque,
                mods::INLINE => K::ModInline,
                mods::OPEN => K::ModOpen,
                mods::INFIX => K::ModInfix,
                mods::IMPLICIT => K::ModImplicit,
                mods::TRANSPARENT => K::ModTransparent,
                _ => K::Other,
            };
            let text = self.slice(span).to_string();
            out.push((F::Mods, self.leaf(kind, span, Some(text))));
        }
        out
    }

    /// A definition's modifiers where they are written, found over the kept tokens from the start of its range (the
    /// index's, which `teq fix` reads with) to its name, as `parse_modifier_run` reads them: past its annotations and
    /// an access modifier's qualifier, a hard modifier, `case` before `class` or `object`, a soft one by its name; the
    /// parser records none of them, so that the compile pays nothing for it.
    fn written_modifiers(&self, d: DefId, def: &Def) -> Vec<(Mods, Span)> {
        let range = self.ast.def_range(d);
        let toks: Vec<(Tok, Span, crate::intern::Name)> = self.tokens[self.tokens.partition_point(|t| t.span.start < range.start)..]
            .iter()
            .take_while(|t| t.span.start < def.span.start)
            .filter_map(|t| super::source::editable_kind(self.text, t).map(|k| (k, t.span, t.name)))
            .collect();
        let mut out = Vec::new();
        let mut i = 0;
        let skip_group = |i: &mut usize| {
            let mut depth = 0u32;
            while *i < toks.len() {
                match toks[*i].0 {
                    Tok::LBracket | Tok::LParen => depth += 1,
                    Tok::RBracket | Tok::RParen => {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            *i += 1;
                            return;
                        }
                    }
                    _ => {}
                }
                *i += 1;
            }
        };
        while i < toks.len() {
            let (k, s, n) = toks[i];
            let bit = match k {
                Tok::At => {
                    // An annotation ends where the parser's `annot()` ended it (`ensureApplied(parArgumentExprss(
                    // wrapNew(simpleType1())))`): the definition's annotation read after this `@`, its instance's end; a
                    // soft modifier after it is the definition's.
                    let end = def.annots.iter().map(|a| self.ast.expr_span(a.instance)).filter(|e| e.start >= s.end).min_by_key(|e| e.start).map(|e| e.end);
                    i += 1;
                    if let Some(end) = end {
                        while i < toks.len() && toks[i].1.start < end {
                            i += 1;
                        }
                    }
                    continue;
                }
                Tok::LBracket => {
                    skip_group(&mut i);
                    continue;
                }
                Tok::KwPrivate => mods::PRIVATE,
                Tok::KwProtected => mods::PROTECTED,
                Tok::KwSealed => mods::SEALED,
                Tok::KwAbstract => mods::ABSTRACT,
                Tok::KwFinal => mods::FINAL,
                Tok::KwLazy => mods::LAZY,
                Tok::KwOverride => mods::OVERRIDE,
                Tok::KwImplicit => mods::IMPLICIT,
                Tok::KwCase if matches!(toks.get(i + 1).map(|t| t.0), Some(Tok::KwClass | Tok::KwObject)) => mods::CASE,
                Tok::Ident => match n {
                    crate::names::OPAQUE => mods::OPAQUE,
                    crate::names::INLINE => mods::INLINE,
                    crate::names::TRANSPARENT => mods::TRANSPARENT,
                    crate::names::OPEN => mods::OPEN,
                    crate::names::INFIX => mods::INFIX,
                    _ => 0,
                },
                _ => 0,
            };
            if bit != 0 {
                out.push((bit, s));
            }
            i += 1;
        }
        out
    }

    fn annot(&mut self, a: &Annot) -> u32 {
        let init = self.init_of(a.instance);
        let span = self.out.nodes.get(init as usize).map_or(Span::default(), |n| n.span);
        // The `@` the annotation is written with, the lexer's token before its type.
        let at = self.tokens.partition_point(|t| t.span.start < span.start);
        let start = match at.checked_sub(1).and_then(|i| self.tokens.get(i)) {
            Some(t) if t.kind == Tok::At => t.span.start,
            _ => span.start,
        };
        self.make(K::ModAnnot, Span::new(start, span.end), vec![(F::Init, init)], None, 0)
    }

    /// An `Init` from a `new T(args)(more)`: an annotation's instance or a parent's constructor call.
    fn init_of(&mut self, e: ExprId) -> u32 {
        let mut lists: Vec<ListRef> = Vec::new();
        let mut cur = e;
        loop {
            match self.ast.expr(cur) {
                Expr::Apply(f, args) | Expr::UsingApply(f, args) => {
                    lists.push(args);
                    cur = f;
                }
                Expr::New(t, args) => {
                    lists.push(args);
                    lists.reverse();
                    let tpe = self.ty(t);
                    let mut fields = vec![(F::Tpe, tpe)];
                    for l in lists {
                        for &a in self.ast.expr_list(l).to_vec().iter() {
                            fields.push((F::Argss, self.expr(a)));
                        }
                    }
                    let span = self.ast.expr_span(e);
                    let tspan = self.out.nodes[tpe as usize].span;
                    return self.make(K::Init, Span::new(tspan.start, span.end), fields, None, 0);
                }
                _ => return self.expr(e),
            }
        }
    }

    fn def(&mut self, d: DefId) -> Option<u32> {
        if self.made_up(d) {
            return None;
        }
        if self.ast.patdef_components.contains(&d) || self.ast.patdef_holders.contains(&d) {
            return self.pattern_def(d);
        }
        let ast: &'a Ast = self.ast;
        let def: &'a Def = ast.def(d);
        let range = ast.def_range(d);
        let mut fields = self.mods(d, def);
        let n = match &def.kind {
            DefKind::Val { pat, ty, rhs } => {
                let mutable = def.mods & mods::MUTABLE != 0;
                let mut names = vec![d];
                let mut next = d;
                // `val a, b = e`: the names after the first, each a copy of the definition (`Ast::val_copies`).
                while let Some((&c, _)) = ast.val_copies.iter().find(|(_, &prev)| prev == next) {
                    names.push(c);
                    next = c;
                }
                match pat {
                    Some(p) => fields.push((F::Pats, self.pat(*p))),
                    None => {
                        for &nd in &names {
                            let nm = ast.def(nd);
                            let pv = if self.slice(nm.span) == "_" {
                                self.leaf(K::PatWildcard, nm.span, Some("_".to_string()))
                            } else {
                                let v = self.name(K::TermName, nm.span, nm.name);
                                self.make(K::PatVar, nm.span, vec![(F::Name, v)], None, 0)
                            };
                            fields.push((F::Pats, pv));
                        }
                    }
                }
                if let Some(t) = ty {
                    fields.push((F::Decltpe, self.ty(*t)));
                }
                let kind = match (rhs, mutable) {
                    (Some(_), false) => K::DefnVal,
                    (Some(_), true) => K::DefnVar,
                    (None, false) => K::DeclVal,
                    (None, true) => K::DeclVar,
                };
                if let Some(r) = rhs {
                    if !ast.default_inits.contains(r) {
                        fields.push((F::Rhs, self.expr(*r)));
                    }
                }
                self.make(kind, range, fields, None, 0)
            }
            DefKind::Fun(f) => {
                let name = self.name(K::TermName, def.span, def.name);
                fields.push((F::Name, name));
                self.type_params(&f.tparams, &mut fields);
                let from = self.out.nodes[name as usize].span.end;
                self.param_clauses(&f.clauses, from, F::Paramss, &mut fields);
                if let Some(t) = f.ret {
                    fields.push((F::Decltpe, self.ty(t)));
                }
                let kind = match f.body {
                    Some(b) => {
                        fields.push((F::Body, self.expr(b)));
                        K::DefnDef
                    }
                    None => K::DeclDef,
                };
                self.make(kind, range, fields, None, 0)
            }
            DefKind::Class(c) => {
                let kind = match c.kind {
                    ClassKind::Class => K::DefnClass,
                    ClassKind::Trait => K::DefnTrait,
                    ClassKind::Object => K::DefnObject,
                    ClassKind::Enum => K::DefnEnum,
                    ClassKind::EnumCase => K::DefnEnumCase,
                };
                let name_kind = if c.kind == ClassKind::Object { K::TermName } else { K::TypeName };
                let name = self.name(name_kind, def.span, def.name);
                fields.push((F::Name, name));
                self.type_params(&c.tparams, &mut fields);
                let from = self.out.nodes[name as usize].span.end;
                self.param_clauses(&c.clauses, from, F::Ctor, &mut fields);
                let header_end = self.out.nodes[name as usize].span.end;
                let templ = self.template(&c.parents, &c.body, c.exports, c.self_type, range, header_end);
                fields.push((F::Templ, templ));
                self.make(kind, range, fields, None, 0)
            }
            DefKind::TypeAlias { tparams, rhs, lower, upper } => {
                let name = self.name(K::TypeName, def.span, def.name);
                fields.push((F::Name, name));
                self.type_params(tparams, &mut fields);
                if let Some(t) = rhs {
                    fields.push((F::Body, self.ty(*t)));
                }
                if let Some(t) = lower {
                    fields.push((F::Lo, self.ty(*t)));
                }
                if let Some(t) = upper {
                    fields.push((F::Hi, self.ty(*t)));
                }
                let kind = if rhs.is_some() { K::DefnType } else { K::DeclType };
                self.make(kind, range, fields, None, 0)
            }
            DefKind::Given(g) => {
                if def.mods & mods::ANONYMOUS == 0 {
                    let name = self.name(K::TermName, def.span, def.name);
                    fields.push((F::Name, name));
                }
                self.type_params(&g.tparams, &mut fields);
                let from = fields.iter().map(|&(_, n)| self.out.nodes[n as usize].span.end).max().unwrap_or(range.start);
                self.param_clauses(&g.clauses, from, F::Paramss, &mut fields);
                fields.push((F::Decltpe, self.ty(g.ty)));
                let kind = match g.alias {
                    Some(a) => {
                        fields.push((F::Body, self.expr(a)));
                        K::DefnGivenAlias
                    }
                    None => {
                        let header_end = fields.iter().map(|&(_, n)| self.out.nodes[n as usize].span.end).max().unwrap_or(range.start);
                        let templ = self.template(&g.parents, &g.body, ListRef::EMPTY, None, range, header_end);
                        fields.push((F::Templ, templ));
                        K::DefnGiven
                    }
                };
                self.make(kind, range, fields, None, 0)
            }
        };
        Some(n)
    }

    fn type_params(&mut self, tps: &[TypeParam], fields: &mut Vec<(F, u32)>) {
        for tp in tps {
            let mut tf: Vec<(F, u32)> = Vec::new();
            // Its `+` or `-`, the token before its name (the parser records none of them).
            let before = self.tokens.partition_point(|t| t.span.start < tp.span.start);
            if let (true, Some(t)) = (tp.variance != 0, before.checked_sub(1).and_then(|i| self.tokens.get(i))) {
                if matches!(self.slice(t.span), "+" | "-") {
                    let k = if tp.variance > 0 { K::ModCovariant } else { K::ModContravariant };
                    let text = self.slice(t.span).to_string();
                    tf.push((F::Mods, self.leaf(k, t.span, Some(text))));
                }
            }
            for a in &tp.annots {
                tf.push((F::Mods, self.annot(a)));
            }
            let name = self.name(K::TypeName, tp.span, tp.name);
            tf.push((F::Name, name));
            self.hk_params(tp.span.start, &mut tf);
            if let Some(t) = tp.lower {
                tf.push((F::Lo, self.ty(t)));
            }
            if let Some(t) = tp.upper {
                tf.push((F::Hi, self.ty(t)));
            }
            for &cb in &tp.context_bounds {
                tf.push((F::Other, self.ty(cb)));
            }
            let node = self.type_param(tp.span, tf);
            fields.push((F::Tparams, node));
        }
    }

    /// A `Type.Param` over its fields, from its first to its last, its clause's `]` included.
    fn type_param(&mut self, name: Span, mut tf: Vec<(F, u32)>) -> u32 {
        tf.retain(|&(_, n)| n != NONE);
        let start = tf.iter().map(|&(_, n)| self.out.nodes[n as usize].span.start).min().unwrap_or(name.start);
        let clause_end = self.ast.notes.hk_clauses.iter().find(|c| c.0 == name.start).map_or(0, |c| c.1);
        let end = tf.iter().map(|&(_, n)| self.out.nodes[n as usize].span.end).max().unwrap_or(name.end).max(clause_end);
        self.make(K::TypeParam, Span::new(start, end), tf, None, 0)
    }

    /// The parameters of the higher-kinded clause that follows the name starting at `owner` (`A` of `F[+A]`), as
    /// dotty's `typeParamClause` keeps it: scalameta's nested `Type.Param`s, each with its variance a modifier, its
    /// own clause and its bounds (a bound the parser makes up for a clause, `[Z] =>> Any`, is no node).
    fn hk_params(&mut self, owner: u32, tf: &mut Vec<(F, u32)>) {
        let params: Vec<crate::ast::HkParam> = self.ast.notes.hk_params.iter().filter(|p| p.owner == owner).copied().collect();
        for p in params {
            let mut f: Vec<(F, u32)> = Vec::new();
            let annots: Vec<crate::ast::Annot> = self.ast.notes.binder_annots.iter().filter(|a| a.name == p.name.start).map(|a| a.annot).collect();
            for a in &annots {
                f.push((F::Mods, self.annot(a)));
            }
            if let Some(s) = p.variance {
                let k = if self.slice(s) == "+" { K::ModCovariant } else { K::ModContravariant };
                let text = self.slice(s).to_string();
                f.push((F::Mods, self.leaf(k, s, Some(text))));
            }
            let written = self.slice(p.name).to_string();
            let kind = if written == "_" { K::NamePlaceholder } else { K::TypeName };
            f.push((F::Name, self.leaf(kind, p.name, Some(written))));
            self.hk_params(p.name.start, &mut f);
            if let Some(t) = p.lower {
                f.push((F::Lo, self.ty(t)));
            }
            if let Some(t) = p.upper {
                f.push((F::Hi, self.ty(t)));
            }
            let cbounds: Vec<(TyExprId, Span)> = self.ast.notes.binder_cbounds.iter().filter(|c| c.0 == p.name.start).map(|c| (c.1, c.2)).collect();
            let named_end = cbounds.iter().map(|c| c.1.end).max().unwrap_or(0);
            for (b, _) in cbounds {
                f.push((F::Other, self.ty(b)));
            }
            let node = self.type_param(p.name, f);
            if named_end > self.out.nodes[node as usize].span.end {
                self.out.nodes[node as usize].span.end = named_end;
            }
            tf.push((F::Tparams, node));
        }
    }

    /// A definition's parameter clauses, `from` the end of its name: each one's `Term.ParamClause`.
    fn param_clauses(&mut self, cls: &[ParamClause], from: u32, f: F, fields: &mut Vec<(F, u32)>) {
        let mut at = from;
        for c in cls {
            let n = self.param_clause(c, at);
            at = at.max(self.out.nodes[n as usize].span.end);
            fields.push((f, n));
        }
    }

    /// A parameter clause, its parentheses in it, as scalameta's: from the `(` before its first parameter to the `)`
    /// after its last, found over the kept tokens (the parser records no clause's span); an empty one the first `()`
    /// at or after `at`, the end of what precedes it.
    fn param_clause(&mut self, c: &ParamClause, at: u32) -> u32 {
        let flags = if c.is_implicit { flags::IMPLICIT } else if c.is_using { flags::USING } else { 0 };
        let mut fields = Vec::new();
        for p in &c.params {
            fields.push((F::Values, self.param(p, flags)));
        }
        let kind = |t: &Token| super::source::editable_kind(self.text, t);
        let first = fields.iter().map(|&(_, n): &(F, u32)| self.out.nodes[n as usize].span.start).min();
        let last = fields.iter().map(|&(_, n): &(F, u32)| self.out.nodes[n as usize].span.end).max();
        let span = match (first, last) {
            (Some(first), Some(last)) => {
                let before = self.tokens.partition_point(|t| t.span.end <= first);
                let open = self.tokens[..before].iter().rev().find(|t| kind(t) == Some(Tok::LParen)).map_or(first, |t| t.span.start);
                let after = self.tokens.partition_point(|t| t.span.start < last);
                let close = self.tokens[after..].iter().find(|t| kind(t) == Some(Tok::RParen)).map_or(last, |t| t.span.end);
                Span::new(open, close)
            }
            _ => {
                // The next tokens at `at`, past a type parameter clause and a constructor's access modifier.
                let mut toks = self.tokens[self.tokens.partition_point(|t| t.span.start < at)..].iter().filter_map(|t| kind(t).map(|k| (k, t.span)));
                let mut depth = 0u32;
                let mut found = None;
                while let Some((k, s)) = toks.next() {
                    match k {
                        Tok::LBracket => depth += 1,
                        Tok::RBracket => depth = depth.saturating_sub(1),
                        _ if depth > 0 => {}
                        Tok::KwPrivate | Tok::KwProtected => {}
                        Tok::LParen => {
                            if let Some((Tok::RParen, close)) = toks.next() {
                                found = Some(Span::new(s.start, close.end));
                            }
                            break;
                        }
                        _ => break,
                    }
                }
                found.unwrap_or(Span::new(at, at))
            }
        };
        self.make(K::ParamClause, span, fields, None, flags)
    }

    fn param(&mut self, p: &Param, clause: u32) -> u32 {
        let mut fields = Vec::new();
        for &a in self.ast.param_annots[p.annots.range()].to_vec().iter() {
            fields.push((F::Mods, self.annot(&a)));
        }
        let name = self.name(K::TermName, p.span, p.name);
        fields.push((F::Name, name));
        let mut flags = clause;
        if p.mods & mods::FIELD != 0 {
            flags |= if p.mods & mods::MUTABLE != 0 { flags::VAR } else { flags::VAL };
        }
        if self.ast.ty_spans.get(p.ty.idx()).is_some_and(|s| s.end > s.start) {
            fields.push((F::Decltpe, self.ty(p.ty)));
        }
        if let Some(d) = p.default {
            fields.push((F::Default, self.expr(d)));
        }
        let start = fields.iter().map(|&(_, n): &(F, u32)| self.out.nodes[n as usize].span.start).min().unwrap_or(p.span.start);
        let end = fields.iter().map(|&(_, n): &(F, u32)| self.out.nodes[n as usize].span.end).max().unwrap_or(p.span.end);
        self.make(K::Param, Span::new(start, end), fields, None, flags)
    }

    /// A class's or a structural given's template: its parents' constructor calls, its self and its statements.
    /// Where a template starts, as scalameta's: from `from` (the end of its definition's name, of a given's type, or
    /// of a `new`), past the rest of the header at bracket depth zero (type parameter and constructor clauses, a
    /// constructor's annotations and access modifier), the first other token, or the one after it where that is
    /// `extends` or a given's `with`; its definition's end where none follows (`object A`). Its braces and an empty
    /// body are in it.
    fn template_start(&self, from: u32, end: u32) -> u32 {
        let toks: Vec<(Tok, Span)> = self.tokens.iter().filter(|t| t.span.start >= from && t.span.start < end).filter_map(|t| super::source::editable_kind(self.text, t).map(|k| (k, t.span))).collect();
        let (mut depth, mut annotation) = (0u32, false);
        for (i, &(k, s)) in toks.iter().enumerate() {
            match k {
                Tok::LParen | Tok::LBracket => {
                    depth += 1;
                    annotation = false;
                }
                Tok::RParen | Tok::RBracket => depth = depth.saturating_sub(1),
                _ if depth > 0 => {}
                Tok::At => annotation = true,
                Tok::Ident | Tok::Dot if annotation => {}
                Tok::KwPrivate | Tok::KwProtected => {}
                Tok::KwExtends | Tok::KwWith => return toks.get(i + 1).map_or(end, |&(_, n)| n.start),
                _ => return s.start,
            }
        }
        end
    }

    fn template(&mut self, parents: &[Parent], body: &[Stmt], exports: ListRef, self_type: Option<TyExprId>, range: Span, header_end: u32) -> u32 {
        let mut fields = Vec::new();
        for p in parents {
            let tpe = self.ty(p.ty);
            if tpe == NONE {
                continue;
            }
            let mut pf = vec![(F::Tpe, tpe)];
            for &(l, _) in &p.args {
                for &a in self.ast.expr_list(l).to_vec().iter() {
                    pf.push((F::Argss, self.expr(a)));
                }
            }
            let tspan = self.out.nodes[tpe as usize].span;
            let end = p.ends.last().copied().filter(|&e| e != NO_END).map_or(tspan.end, |e| e + 1);
            fields.push((F::Inits, self.make(K::Init, Span::new(tspan.start, end), pf, None, 0)));
        }
        if let Some(t) = self_type {
            let tpe = self.ty(t);
            fields.push((F::SelfDecl, tpe));
        }
        for n in self.stmts(body) {
            fields.push((F::Stats, n));
        }
        let exported: Vec<&Import> = self.ast.exports[exports.range()].iter().collect();
        for n in self.import_statements(&exported, &[], K::Export) {
            fields.push((F::Stats, n));
        }
        let start = self.template_start(header_end, range.end);
        self.make(K::Template, Span::new(start, range.end), fields, None, 0)
    }

    fn stmts(&mut self, body: &[Stmt]) -> Vec<u32> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < body.len() {
            match body[i] {
                Stmt::Expr(e) => out.push(self.expr(e)),
                Stmt::Def(d) => {
                    if let Some(n) = self.def(d) {
                        out.push(n);
                    }
                }
                Stmt::Import(k) => {
                    let l = self.ast.import_stmts[k as usize];
                    let sels: Vec<&Import> = self.ast.local_imports[l.range()].iter().collect();
                    out.extend(self.import_statements(&sels, &[], K::Import));
                }
            }
            i += 1;
        }
        out
    }

    // ---- terms ----

    fn exprs(&mut self, l: ListRef, f: F) -> Vec<(F, u32)> {
        self.ast.expr_list(l).to_vec().into_iter().map(|e| (f, self.expr(e))).collect()
    }

    fn expr(&mut self, e: ExprId) -> u32 {
        let ast: &'a Ast = self.ast;
        let span = ast.expr_span(e);
        if span.end < span.start || span == Span::default() && !matches!(ast.expr(e), Expr::UnitLit) {
            return NONE;
        }
        let lit = |b: &mut Self, k: K| {
            let text = b.slice(span).to_string();
            b.leaf(k, span, Some(text))
        };
        match ast.expr(e) {
            Expr::IntLit(_) => lit(self, K::LitInt),
            Expr::LongLit(_) => lit(self, K::LitLong),
            Expr::DoubleLit(_) | Expr::DecimalLit(_) => lit(self, K::LitDouble),
            Expr::FloatLit(_) => lit(self, K::LitFloat),
            Expr::BoolLit(_) => lit(self, K::LitBoolean),
            Expr::CharLit(_) => lit(self, K::LitChar),
            Expr::StringLit(s) => {
                let value = ast.str(s).to_string();
                self.leaf(K::LitString, span, Some(value))
            }
            Expr::UnitLit => lit(self, K::LitUnit),
            Expr::NullLit => lit(self, K::LitNull),
            // `_` of a placeholder function (`_ + 1`), scalameta's `Term.Placeholder`.
            Expr::Ident(_) if self.slice(span) == "_" => self.leaf(K::TermPlaceholder, span, Some("_".to_string())),
            Expr::Ident(n) => self.name(K::TermName, span, n),
            Expr::This | Expr::QualThis(_) => lit(self, K::TermThis),
            Expr::Super(_) => lit(self, K::TermSuper),
            Expr::Select(q, n) => {
                let qual = self.expr(q);
                let name = match ast.name_span(e) {
                    Some(s) => self.name(K::TermName, s, n),
                    None => NONE,
                };
                self.make(K::TermSelect, span, vec![(F::Qual, qual), (F::Name, name)], None, 0)
            }
            Expr::Apply(f, args) | Expr::UsingApply(f, args) => {
                let kind = if matches!(ast.expr(e), Expr::UsingApply(..)) { K::TermApplyUsing } else { K::TermApply };
                let mut fields = vec![(F::Fun, self.expr(f))];
                fields.extend(self.exprs(args, F::Args));
                self.make(kind, span, fields, None, 0)
            }
            Expr::NamedArg(n, v) => {
                let name = self.name_from(K::TermName, span.start, n);
                let rhs = self.expr(v);
                self.make(K::TermAssign, span, vec![(F::Lhs, name), (F::Rhs, rhs)], None, 0)
            }
            Expr::TypeApply(f, targs) => {
                let mut fields = vec![(F::Fun, self.expr(f))];
                for t in ast.ty_list(targs).to_vec() {
                    fields.push((F::Targs, self.ty(t)));
                }
                self.make(K::TermApplyType, span, fields, None, 0)
            }
            Expr::Infix(l, op, r, _) => {
                let lhs = self.expr(l);
                let opn = match ast.name_span(e) {
                    Some(s) => self.name(K::TermName, s, op),
                    None => NONE,
                };
                let mut fields = vec![(F::Lhs, lhs), (F::Op, opn)];
                match ast.expr(r) {
                    Expr::Tuple(xs) => fields.extend(self.exprs(xs, F::Args)),
                    _ => fields.push((F::Args, self.expr(r))),
                }
                self.make(K::TermApplyInfix, span, fields, None, 0)
            }
            Expr::Prefix(op, x) => {
                let opn = self.name_from(K::TermName, span.start, op);
                let arg = self.expr(x);
                self.make(K::TermApplyUnary, span, vec![(F::Op, opn), (F::Arg, arg)], None, 0)
            }
            Expr::Lambda(params, body) if !params.is_empty() && {
                let b = ast.expr_span(body);
                ast.lambda_params[params.range()].iter().all(|p| b.start <= p.span.start && p.span.end <= b.end)
            } =>
            {
                // A placeholder function (`_ + 1`): the parser's parameters stand at its `_`s, inside its body.
                let b = self.expr(body);
                self.make(K::TermAnonymousFunction, span, vec![(F::Body, b)], None, 0)
            }
            Expr::Lambda(params, body) => {
                // A polymorphic function literal's evidences (`poly_lambda_evidence`) stand at their bounds, outside
                // the lambda: no parameter of it, and a lambda of them alone no node.
                let all = ast.lambda_params[params.range()].to_vec();
                let own: Vec<crate::ast::LambdaParam> = all.iter().copied().filter(|p| span.start <= p.span.start && p.span.end <= span.end).collect();
                if own.is_empty() && !all.is_empty() {
                    return self.expr(body);
                }
                let mut fields = Vec::new();
                for p in own {
                    let name = self.name(K::TermName, p.span, p.name);
                    let mut pf = vec![(F::Name, name)];
                    if let Some(t) = p.ty {
                        pf.push((F::Decltpe, self.ty(t)));
                    }
                    let flags = if p.ty.is_none() { flags::UNTYPED } else { 0 } | if p.implicit { flags::IMPLICIT } else { 0 } | if p.contextual { flags::USING } else { 0 };
                    let end = pf.iter().map(|&(_, n)| self.out.nodes.get(n as usize).map_or(p.span.end, |x| x.span.end)).max().unwrap_or(p.span.end);
                    let param = self.make(K::Param, Span::new(p.span.start, end), pf, None, flags);
                    fields.push((F::Params, param));
                }
                fields.push((F::Body, self.expr(body)));
                self.make(K::TermFunction, span, fields, None, 0)
            }
            Expr::PolyLambda(_, body) => {
                let mut fields = Vec::new();
                self.hk_params(span.start, &mut fields);
                fields.push((F::Body, self.expr(body)));
                self.make(K::TermPolyFunction, span, fields, None, 0)
            }
            Expr::If(c, t, f) | Expr::InlineIf(c, t, f) => {
                let mut fields = vec![(F::Cond, self.expr(c)), (F::Thenp, self.expr(t))];
                if let Some(f) = f {
                    fields.push((F::Elsep, self.expr(f)));
                }
                self.make(K::TermIf, span, fields, None, 0)
            }
            Expr::Match(s, cases) | Expr::InlineMatch(s, cases) => {
                let mut fields = vec![(F::Expr, self.expr(s))];
                for k in cases.range() {
                    fields.push((F::Cases, self.case(k)));
                }
                self.make(K::TermMatch, span, fields, None, 0)
            }
            Expr::Block(stmts) => {
                let body: Vec<Stmt> = ast.stmt_list(stmts).to_vec();
                let fields = self.stmts(&body).into_iter().map(|n| (F::Stats, n)).collect();
                self.make(K::TermBlock, span, fields, None, 0)
            }
            Expr::While(c, b) => {
                let fields = vec![(F::Cond, self.expr(c)), (F::Body, self.expr(b))];
                self.make(K::TermWhile, span, fields, None, 0)
            }
            Expr::For(enums, body, yields) => {
                let mut fields = Vec::new();
                for k in enums.range() {
                    fields.push((F::Enums, self.enumerator(k)));
                }
                fields.push((F::Body, self.expr(body)));
                self.make(if yields { K::TermForYield } else { K::TermFor }, span, fields, None, 0)
            }
            Expr::Assign(l, r) => {
                let fields = vec![(F::Lhs, self.expr(l)), (F::Rhs, self.expr(r))];
                self.make(K::TermAssign, span, fields, None, 0)
            }
            Expr::Tuple(xs) => {
                let fields = self.exprs(xs, F::Args);
                self.make(K::TermTuple, span, fields, None, 0)
            }
            Expr::NamedTuple(_, values) => {
                let fields = self.exprs(values, F::Args);
                self.make(K::TermTuple, span, fields, None, 0)
            }
            Expr::Parens(x) => {
                let inner = self.expr(x);
                self.make(K::TermParens, span, vec![(F::Expr, inner)], None, 0)
            }
            Expr::Typed(x, t) => {
                let fields = vec![(F::Expr, self.expr(x)), (F::Tpe, self.ty(t))];
                self.make(K::TermAscribe, span, fields, None, 0)
            }
            Expr::Unchecked(x) => {
                let inner = self.expr(x);
                self.make(K::TermAscribe, span, vec![(F::Expr, inner)], None, 0)
            }
            Expr::SeqLit(xs, _) => {
                let fields = self.exprs(xs, F::Args);
                self.make(K::TermRepeated, span, fields, None, 0)
            }
            Expr::New(..) => {
                let init = self.init_of(e);
                self.make(K::TermNew, span, vec![(F::Init, init)], None, 0)
            }
            Expr::NewAnon(d) => {
                let def: &'a Def = ast.def(d);
                let fields = match &def.kind {
                    DefKind::Class(c) => {
                        // After `new`.
                        let templ = self.template(&c.parents, &c.body, c.exports, c.self_type, span, span.start + 3);
                        vec![(F::Templ, templ)]
                    }
                    _ => Vec::new(),
                };
                self.make(K::TermNewAnonymous, span, fields, None, 0)
            }
            Expr::Interp(id, _, args) => {
                let prefix = self.name_from(K::TermName, span.start, id);
                let mut fields = vec![(F::Prefix, prefix)];
                fields.extend(self.exprs(args, F::Args));
                self.make(K::TermInterpolate, span, fields, None, 0)
            }
            Expr::Return(x) => {
                let fields = x.map(|x| vec![(F::Expr, self.expr(x))]).unwrap_or_default();
                self.make(K::TermReturn, span, fields, None, 0)
            }
            Expr::Throw(x) => {
                let inner = self.expr(x);
                self.make(K::TermThrow, span, vec![(F::Expr, inner)], None, 0)
            }
            Expr::Try(k) => {
                let t = ast.tries[k as usize];
                let mut fields = vec![(F::Expr, self.expr(t.body))];
                for c in t.cases.range() {
                    fields.push((F::Catchp, self.case(c)));
                }
                if let Some(h) = t.handler {
                    fields.push((F::Catchp, self.expr(h)));
                }
                if let Some(f) = t.finalizer {
                    fields.push((F::Finallyp, self.expr(f)));
                }
                self.make(K::TermTry, span, fields, None, 0)
            }
            Expr::Quote(x) => {
                let inner = self.expr(x);
                self.make(K::TermQuote, span, vec![(F::Body, inner)], None, 0)
            }
            Expr::Splice(x) => {
                let inner = self.expr(x);
                self.make(K::TermSplice, span, vec![(F::Body, inner)], None, 0)
            }
            Expr::AsFunction(x) => {
                let inner = self.expr(x);
                self.make(K::TermEta, span, vec![(F::Expr, inner)], None, 0)
            }
            Expr::QuoteType(t) | Expr::ClassOf(t) | Expr::ThisOf(t) => {
                let inner = self.ty(t);
                self.make(K::Other, span, vec![(F::Other, inner)], None, 0)
            }
            Expr::SplicePat(p) => {
                let inner = self.pat(p);
                self.make(K::Other, span, vec![(F::Other, inner)], None, 0)
            }
            Expr::Derived | Expr::Error | Expr::SymRef(_) | Expr::Unsupported(_) | Expr::Withheld(_) => self.leaf(K::Other, span, None),
        }
    }

    fn case(&mut self, k: usize) -> u32 {
        let c = self.ast.cases[k];
        let mut fields = vec![(F::Pat, self.pat(c.pat))];
        if let Some(g) = c.guard {
            fields.push((F::Cond, self.expr(g)));
        }
        fields.push((F::Body, self.expr(c.body)));
        let start = self.out.nodes.get(fields[0].1 as usize).map_or(0, |n| n.span.start);
        // The case's keyword, the lexer's token before its pattern.
        let at = self.tokens.partition_point(|t| t.span.start < start);
        let start = match at.checked_sub(1).and_then(|i| self.tokens.get(i)) {
            Some(t) if t.kind == Tok::KwCase => t.span.start,
            _ => start,
        };
        let end = fields.iter().filter(|&&(_, n)| n != NONE).map(|&(_, n)| self.out.nodes[n as usize].span.end).max().unwrap_or(start);
        self.make(K::Case, Span::new(start, end), fields, None, 0)
    }

    fn enumerator(&mut self, k: usize) -> u32 {
        let (kind, fields) = match self.ast.enumerators[k] {
            Enumerator::Gen(p, e, _) => (K::EnumGenerator, vec![(F::Pat, self.pat(p)), (F::Rhs, self.expr(e))]),
            Enumerator::CaseGen(p, e) => (K::EnumCaseGenerator, vec![(F::Pat, self.pat(p)), (F::Rhs, self.expr(e))]),
            Enumerator::Guard(e) => (K::EnumGuard, vec![(F::Cond, self.expr(e))]),
            Enumerator::Val(p, e) => (K::EnumVal, vec![(F::Pat, self.pat(p)), (F::Rhs, self.expr(e))]),
        };
        let spans: Vec<Span> = fields.iter().filter(|&&(_, n)| n != NONE).map(|&(_, n)| self.out.nodes[n as usize].span).collect();
        let span = Span::new(spans.iter().map(|s| s.start).min().unwrap_or(0), spans.iter().map(|s| s.end).max().unwrap_or(0));
        self.make(kind, span, fields, None, 0)
    }

    // ---- patterns ----

    fn pat(&mut self, p: PatId) -> u32 {
        let ast = self.ast;
        let span = ast.pat_spans[p.idx()];
        match ast.pat(p) {
            Pat::Wildcard => self.leaf(K::PatWildcard, span, Some("_".to_string())),
            Pat::Bind(n, None) => {
                let name = self.name(K::TermName, span, n);
                self.make(K::PatVar, span, vec![(F::Name, name)], None, 0)
            }
            Pat::Bind(n, Some(inner)) => {
                let name = self.name_from(K::TermName, span.start, n);
                let var = match name {
                    NONE => NONE,
                    _ => {
                        let s = self.out.nodes[name as usize].span;
                        self.make(K::PatVar, s, vec![(F::Name, name)], None, 0)
                    }
                };
                let rhs = self.pat(inner);
                self.make(K::PatBind, span, vec![(F::Lhs, var), (F::Rhs, rhs)], None, 0)
            }
            Pat::Typed(inner, t) => {
                let fields = vec![(F::Lhs, self.pat(inner)), (F::Rhs, self.ty(t))];
                self.make(K::PatTyped, span, fields, None, 0)
            }
            Pat::Lit(e) | Pat::StableId(e) => self.expr(e),
            Pat::Ctor(f, args) => {
                let fun = self.expr(f);
                let infix = fun != NONE && self.out.nodes[fun as usize].span.start > span.start;
                let mut fields = vec![(if infix { F::Op } else { F::Fun }, fun)];
                for a in ast.pat_list(args).to_vec() {
                    fields.push((F::Args, self.pat(a)));
                }
                if infix {
                    // `h :: t`: the first operand is the left one.
                    if let Some(i) = fields.iter().position(|f| f.0 == F::Args) {
                        fields[i].0 = F::Lhs;
                    }
                }
                self.make(if infix { K::PatExtractInfix } else { K::PatExtract }, span, fields, None, 0)
            }
            Pat::Tuple(xs) => {
                let fields = ast.pat_list(xs).to_vec().into_iter().map(|x| (F::Args, self.pat(x))).collect();
                self.make(K::PatTuple, span, fields, None, 0)
            }
            Pat::NamedField(_, x) => {
                let inner = self.pat(x);
                self.make(K::Other, span, vec![(F::Other, inner)], None, 0)
            }
            Pat::Alt(xs) => {
                let fields = ast.pat_list(xs).to_vec().into_iter().map(|x| (F::Args, self.pat(x))).collect();
                self.make(K::PatAlternative, span, fields, None, 0)
            }
            Pat::Rest(_) => self.leaf(K::PatSeqWildcard, span, None),
            Pat::Interp(id, _, args) => {
                let prefix = self.name_from(K::TermName, span.start, id);
                let mut fields = vec![(F::Prefix, prefix)];
                for a in ast.pat_list(args).to_vec() {
                    fields.push((F::Args, self.pat(a)));
                }
                self.make(K::PatInterpolate, span, fields, None, 0)
            }
            Pat::Quote(e) => {
                let inner = self.expr(e);
                self.make(K::Other, span, vec![(F::Other, inner)], None, 0)
            }
            Pat::QuoteType(t) => {
                let inner = self.ty(t);
                self.make(K::Other, span, vec![(F::Other, inner)], None, 0)
            }
            Pat::Error => self.leaf(K::Other, span, None),
        }
    }

    // ---- types ----

    fn tys(&mut self, l: ListRef, f: F) -> Vec<(F, u32)> {
        self.ast.ty_list(l).to_vec().into_iter().map(|t| (f, self.ty(t))).collect()
    }

    fn ty(&mut self, t: TyExprId) -> u32 {
        let ast = self.ast;
        let Some(&span) = ast.ty_spans.get(t.idx()) else { return NONE };
        if span.end <= span.start {
            return NONE;
        }
        // A lambda the parser made, no tree of the source: a higher-kinded parameter's bound over its clause, its
        // written bound inside it (`G[X] <: Upper[X]`) or none (`[Z] =>> Any`), or a kind projector's over what it
        // applies. Its written body stands for it; the clause's parameters are the parameter's (`hk_params`).
        if let TyExpr::Lambda(_, body) = ast.ty(t) {
            if !ast.notes.written_lambdas.contains(&t) {
                return if matches!(ast.ty(body), TyExpr::Resolved(_)) { NONE } else { self.ty(body) };
            }
        }
        match ast.ty(t) {
            TyExpr::Name(n) => self.name(K::TypeName, span, n),
            TyExpr::Select(q, n) => {
                let qual = self.ty_path(q);
                let name = match ast.ty_name_span(t) {
                    Some(s) => self.name(K::TypeName, s, n),
                    None => NONE,
                };
                self.make(K::TypeSelect, span, vec![(F::Qual, qual), (F::Name, name)], None, 0)
            }
            TyExpr::Apply(f, args) => {
                let mut fields = vec![(F::Tpe, self.ty(f))];
                fields.extend(self.tys(args, F::Args));
                self.make(K::TypeApply, span, fields, None, 0)
            }
            TyExpr::Fun(params, res) | TyExpr::CtxFun(params, res) => {
                // A polymorphic function's context bounds' evidences, which the parser adds as a context function's
                // parameters (`poly_fun_type_evidence`), stand at their bounds, outside the function: no parameter
                // of it, and a context function of them alone no node.
                let written: Vec<TyExprId> = ast.ty_list(params).iter().copied().filter(|p| ast.ty_spans.get(p.idx()).is_some_and(|s| span.start <= s.start && s.end <= span.end)).collect();
                if written.is_empty() && !ast.ty_list(params).is_empty() {
                    return self.ty(res);
                }
                let kind = if matches!(ast.ty(t), TyExpr::CtxFun(..)) { K::TypeContextFunction } else { K::TypeFunction };
                let mut fields: Vec<(F, u32)> = written.into_iter().map(|p| (F::Params, self.ty(p))).collect();
                fields.push((F::Body, self.ty(res)));
                self.make(kind, span, fields, None, 0)
            }
            TyExpr::Tuple(xs) => {
                let fields = self.tys(xs, F::Args);
                self.make(K::TypeTuple, span, fields, None, 0)
            }
            TyExpr::NamedTuple(_, xs) => {
                let fields = self.tys(xs, F::Args);
                self.make(K::TypeTuple, span, fields, None, 0)
            }
            TyExpr::Union(a, b) | TyExpr::Inter(a, b) => {
                let kind = if matches!(ast.ty(t), TyExpr::Union(..)) { K::TypeOr } else { K::TypeAnd };
                let fields = vec![(F::Lhs, self.ty(a)), (F::Rhs, self.ty(b))];
                self.make(kind, span, fields, None, 0)
            }
            TyExpr::ByName(x) => {
                let inner = self.ty(x);
                self.make(K::TypeByName, span, vec![(F::Tpe, inner)], None, 0)
            }
            TyExpr::Repeated(x) => {
                let inner = self.ty(x);
                self.make(K::TypeRepeated, span, vec![(F::Tpe, inner)], None, 0)
            }
            TyExpr::Wildcard => self.leaf(K::TypeWildcard, span, None),
            TyExpr::BoundedWildcard(lo, hi) => {
                let fields = vec![(F::Lo, self.ty(lo)), (F::Hi, self.ty(hi))];
                self.make(K::TypeWildcard, span, fields, None, 0)
            }
            TyExpr::Lambda(_, body) => {
                let mut fields = Vec::new();
                self.hk_params(span.start, &mut fields);
                fields.push((F::Body, self.ty(body)));
                self.make(K::TypeLambda, span, fields, None, 0)
            }
            TyExpr::PolyFun(_, body) => {
                let mut fields = Vec::new();
                self.hk_params(span.start, &mut fields);
                fields.push((F::Body, self.ty(body)));
                self.make(K::TypePolyFunction, span, fields, None, 0)
            }
            TyExpr::Singleton(x) => {
                let inner = self.ty_path(x);
                self.make(K::TypeSingleton, span, vec![(F::Ref, inner)], None, 0)
            }
            TyExpr::Project(q, n) => {
                let qual = self.ty(q);
                let name = match ast.ty_name_span(t) {
                    Some(s) => self.name(K::TypeName, s, n),
                    None => NONE,
                };
                self.make(K::TypeProject, span, vec![(F::Qual, qual), (F::Name, name)], None, 0)
            }
            TyExpr::Refined(parent, decls) => {
                let mut fields = vec![(F::Tpe, self.ty(parent))];
                for d in ast.def_list(decls).to_vec() {
                    if let Some(n) = self.def(d) {
                        fields.push((F::Stats, n));
                    }
                }
                self.make(K::TypeRefine, span, fields, None, 0)
            }
            TyExpr::Lit(e) => self.expr(e),
            TyExpr::Annotated(x, _) | TyExpr::Unchecked(x) | TyExpr::UncheckedVariance(x) => {
                // scalameta's `Type.Annotate(tpe, annots)`: the annotation, its constructor and arguments, a child
                // where it was written (`ast.inline_annots`): by the class an `Annotated` names, by name for the two
                // the parser reads as `Unchecked` and `UncheckedVariance`.
                let inner = self.ty(x);
                let after = ast.ty_spans.get(x.idx()).map_or(span.start, |s| s.end);
                let found = ast.inline_annots.iter().find(|a| match ast.ty(t) {
                    TyExpr::Annotated(_, Some(class)) => ast.annot_new(a).is_some_and(|(path, _)| path == class),
                    TyExpr::Unchecked(_) | TyExpr::UncheckedVariance(_) => {
                        let wanted = if matches!(ast.ty(t), TyExpr::Unchecked(_)) { crate::names::UNCHECKED } else { crate::names::UNCHECKED_VARIANCE };
                        let at = ast.expr_span(a.instance);
                        a.name == wanted && at.start >= after && at.end <= span.end
                    }
                    _ => false,
                });
                let mut fields = vec![(F::Tpe, inner)];
                let mut end = span.end;
                if let Some(a) = found.copied() {
                    let node = self.annot(&a);
                    if node != NONE {
                        end = self.out.nodes[node as usize].span.end;
                        fields.push((F::Annots, node));
                    }
                }
                self.make(K::TypeAnnotate, Span::new(span.start, end.max(after)), fields, None, 0)
            }
            TyExpr::Match(s, cases) => {
                let mut fields = vec![(F::Tpe, self.ty(s))];
                fields.extend(self.tys(cases, F::Cases));
                self.make(K::TypeMatch, span, fields, None, 0)
            }
            TyExpr::MatchCase(a, b) => {
                let fields = vec![(F::Pat, self.ty(a)), (F::Body, self.ty(b))];
                self.make(K::TypeCase, span, fields, None, 0)
            }
            TyExpr::TypeVar(n) => self.name(K::TypeVar, span, n),
            TyExpr::Resolved(_) | TyExpr::MainParam(..) | TyExpr::Error => self.leaf(K::Other, span, None),
        }
    }

    /// A type's qualifier, a path of terms (`p.q` in `p.q.T`).
    fn ty_path(&mut self, t: TyExprId) -> u32 {
        let ast = self.ast;
        let Some(&span) = ast.ty_spans.get(t.idx()) else { return NONE };
        match ast.ty(t) {
            TyExpr::Name(n) => self.name(K::TermName, span, n),
            TyExpr::Select(q, n) => {
                let qual = self.ty_path(q);
                let name = match ast.ty_name_span(t) {
                    Some(s) => self.name(K::TermName, s, n),
                    None => NONE,
                };
                self.make(K::TermSelect, span, vec![(F::Qual, qual), (F::Name, name)], None, 0)
            }
            _ => self.ty(t),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(text: &str) -> Tree {
        let interner = crate::intern::Interner::new();
        let (asts, found) = crate::frontend::parse_one(text, &interner, crate::parser::Syntax { keep: true, index: true, ..Default::default() });
        assert!(found.errors.is_empty(), "{:?}", found.errors);
        let refs: Vec<&Ast> = asts.iter().collect();
        build(text, &refs, refs[0].notes.source.as_deref().unwrap(), &interner)
    }

    /// The tree as text: each node's kind, its text or its value, its children indented.
    fn show(t: &Tree, text: &str, n: u32, depth: usize, out: &mut String) {
        let node = t.nodes[n as usize];
        let s = &text[node.span.start as usize..node.span.end as usize];
        out.push_str(&format!("{}{} `{}`\n", "  ".repeat(depth), node.kind.name(), s.replace('\n', "⏎")));
        for &(_, c) in t.children(n) {
            show(t, text, c, depth + 1, out);
        }
    }

    #[test]
    fn scalameta_shapes() {
        let text = "package p\nimport cats.implicits._\nobject O {\n  final val x = List(1).map(_ => ())\n  def f(a: Int = 1)(implicit b: Int): Unit = a match { case Nil => () }\n}\n";
        let t = tree(text);
        let mut out = String::new();
        show(&t, text, (t.nodes.len() - 1) as u32, 0, &mut out);
        let expected = "Source `package p⏎import cats.implicits._⏎object O {⏎  final val x = List(1).map(_ => ())⏎  def f(a: Int = 1)(implicit b: Int): Unit = a match { case Nil => () }⏎}⏎`
  Pkg `package p⏎import cats.implicits._⏎object O {⏎  final val x = List(1).map(_ => ())⏎  def f(a: Int = 1)(implicit b: Int): Unit = a match { case Nil => () }⏎}`
    Term.Name `p`
    Import `import cats.implicits._`
      Importer `cats.implicits._`
        Term.Select `cats.implicits`
          Term.Name `cats`
          Term.Name `implicits`
        Importee.Wildcard `_`
    Defn.Object `object O {⏎  final val x = List(1).map(_ => ())⏎  def f(a: Int = 1)(implicit b: Int): Unit = a match { case Nil => () }⏎}`
      Term.Name `O`
      Template `{⏎  final val x = List(1).map(_ => ())⏎  def f(a: Int = 1)(implicit b: Int): Unit = a match { case Nil => () }⏎}`
        Defn.Val `final val x = List(1).map(_ => ())`
          Mod.Final `final`
          Pat.Var `x`
            Term.Name `x`
          Term.Apply `List(1).map(_ => ())`
            Term.Select `List(1).map`
              Term.Apply `List(1)`
                Term.Name `List`
                Lit.Int `1`
              Term.Name `map`
            Term.Function `_ => ()`
              Term.Param `_`
                Name.Placeholder `_`
              Lit.Unit `()`
        Defn.Def `def f(a: Int = 1)(implicit b: Int): Unit = a match { case Nil => () }`
          Term.Name `f`
          Term.ParamClause `(a: Int = 1)`
            Term.Param `a: Int = 1`
              Term.Name `a`
              Type.Name `Int`
              Lit.Int `1`
          Term.ParamClause `(implicit b: Int)`
            Term.Param `b: Int`
              Term.Name `b`
              Type.Name `Int`
          Type.Name `Unit`
          Term.Match `a match { case Nil => () }`
            Term.Name `a`
            Case `case Nil => ()`
              Term.Name `Nil`
              Lit.Unit `()`
";
        assert_eq!(out, expected, "\n{}", out);
    }

    /// Every case file's tree: each node inside its parent, the children in source order, every parent set.
    #[test]
    fn nested_over_the_cases() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        collect(&root.join("tests/cases"), &mut files);
        collect(&root.join("tests/fix"), &mut files);
        let mut checked = 0;
        for f in files {
            let Ok(text) = std::fs::read_to_string(&f) else { continue };
            let interner = crate::intern::Interner::new();
            let (asts, found) = crate::frontend::parse_one(&text, &interner, crate::parser::Syntax { keep: true, index: true, ..Default::default() });
            if !found.errors.is_empty() {
                continue;
            }
            let refs: Vec<&Ast> = asts.iter().collect();
            let t = build(&text, &refs, refs[0].notes.source.as_deref().unwrap(), &interner);
            let root_id = (t.nodes.len() - 1) as u32;
            for (i, n) in t.nodes.iter().enumerate() {
                assert!(n.span.start <= n.span.end && n.span.end as usize <= text.len(), "{}: node {} {} {:?}", f.display(), i, n.kind.name(), n.span);
                if i as u32 != root_id {
                    assert!(n.parent != NONE, "{}: node {} {} {:?} has no parent", f.display(), i, n.kind.name(), n.span);
                }
                let kids = t.children(i as u32);
                for w in kids.windows(2) {
                    assert!(t.nodes[w[0].1 as usize].span.start <= t.nodes[w[1].1 as usize].span.start, "{}: children of {} out of order", f.display(), n.kind.name());
                }
            }
            checked += 1;
        }
        assert!(checked > 1000, "{} files", checked);
    }

    fn collect(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                collect(&p, out);
            } else if p.extension().is_some_and(|x| x == "scala") {
                out.push(p);
            }
        }
    }
}
