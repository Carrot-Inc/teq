//! The tree a rule reads of a file (docs/FIX.md, "The tree"): scalameta 4.17.4's parser's (`parse`, over the
//! document's pieces and the scanner's stream), each node of its class and its fields, with its span, its parent, its
//! children by field, its value, its flags and the comments the parser gives it.

use crate::source::Span;

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
    TermApplyType = "Term.ApplyType", TermApplyInfix = "Term.ApplyInfix",
    TermApplyUnary = "Term.ApplyUnary", TermAssign = "Term.Assign", TermBlock = "Term.Block", TermIf = "Term.If",
    TermMatch = "Term.Match", TermTry = "Term.Try", TermWhile = "Term.While", TermFor = "Term.For",
    TermForYield = "Term.ForYield", TermFunction = "Term.Function", TermAnonymousFunction = "Term.AnonymousFunction",
    TermPolyFunction = "Term.PolyFunction",
    TermReturn = "Term.Return", TermThrow = "Term.Throw", TermNew = "Term.New", TermNewAnonymous = "Term.NewAnonymous",
    TermInterpolate = "Term.Interpolate", TermTuple = "Term.Tuple", TermAscribe = "Term.Ascribe",
    TermQuote = "Term.QuotedMacroExpr", TermSplice = "Term.SplicedMacroExpr",
    TermEta = "Term.Eta", TermRepeated = "Term.Repeated",
    LitInt = "Lit.Int", LitLong = "Lit.Long", LitDouble = "Lit.Double", LitFloat = "Lit.Float",
    LitBoolean = "Lit.Boolean", LitChar = "Lit.Char", LitString = "Lit.String", LitUnit = "Lit.Unit", LitNull = "Lit.Null",
    Case = "Case", EnumGenerator = "Enumerator.Generator", EnumCaseGenerator = "Enumerator.CaseGenerator",
    EnumGuard = "Enumerator.Guard", EnumVal = "Enumerator.Val",
    PatWildcard = "Pat.Wildcard", PatVar = "Pat.Var", PatBind = "Pat.Bind", PatTyped = "Pat.Typed",
    PatExtract = "Pat.Extract", PatExtractInfix = "Pat.ExtractInfix", PatTuple = "Pat.Tuple",
    PatAlternative = "Pat.Alternative", PatSeqWildcard = "Pat.SeqWildcard", PatInterpolate = "Pat.Interpolate",
    TypeSelect = "Type.Select", TypeApply = "Type.Apply", TypeFunction = "Type.Function",
    TypeContextFunction = "Type.ContextFunction", TypeTuple = "Type.Tuple",
    TypeByName = "Type.ByName", TypeRepeated = "Type.Repeated", TypeWildcard = "Type.Wildcard",
    TypeLambda = "Type.Lambda", TypePolyFunction = "Type.PolyFunction", TypeSingleton = "Type.Singleton",
    TypeProject = "Type.Project", TypeRefine = "Type.Refine", TypeAnnotate = "Type.Annotate", TypeMatch = "Type.Match",
    TypeCase = "TypeCase", TypeVar = "Type.Var",
    PkgBody = "Pkg.Body", TemplateBody = "Template.Body", SelfNode = "Self", NameIndeterminate = "Name.Indeterminate",
    NameAnonymous = "Name.Anonymous",
    NameThis = "Name.This", TermArgClause = "Term.ArgClause", TermCasesBlock = "Term.CasesBlock",
    TermEnumeratorsBlock = "Term.EnumeratorsBlock", CtorPrimary = "Ctor.Primary", CtorSecondary = "Ctor.Secondary",
    CtorBlock = "Ctor.Block", ParamClauseGroup = "Member.ParamClauseGroup", TypeParamClause = "Type.ParamClause",
    PatArgClause = "Pat.ArgClause", TermPartialFunction = "Term.PartialFunction", TermSelectMatch = "Term.SelectMatch",
    TermSelectPostfix = "Term.SelectPostfix", TermTryWithHandler = "Term.TryWithHandler",
    TermContextFunction = "Term.ContextFunction", DefnExtensionGroup = "Defn.ExtensionGroup",
    DefnRepeatedEnumCase = "Defn.RepeatedEnumCase", ModValParam = "Mod.ValParam", ModVarParam = "Mod.VarParam",
    ModUsing = "Mod.Using", TypeArgClause = "Type.ArgClause", TypeBounds = "Type.Bounds",
    TypeFuncParamClause = "Type.FuncParamClause", TypeTypedParam = "Type.TypedParam", TypeApplyInfix = "Type.ApplyInfix", TermAnnotate = "Term.Annotate",
    TypeCasesBlock = "Type.CasesBlock", TypePatWildcard = "Type.PatWildcard", StatBlock = "Stat.Block",
    PatRepeated = "Pat.Repeated", PatGiven = "Pat.Given", TermEndMarker = "Term.EndMarker",
    LitWithUnary = "Lit.WithUnary", TypeWith = "Type.With", TypeBoundsAlias = "Type.BoundsAlias",
    TermQuotedType = "Term.QuotedMacroType", TermSplicePat = "Term.SplicedMacroPat", TermAnonymous = "Term.Anonymous",
    PatMacro = "Pat.Macro", PatAssign = "Pat.Assign", PkgObject = "Pkg.Object", DeclGiven = "Decl.Given", DeclGivenAnonymous = "Decl.GivenAnonymous",
    TypeAnonymousLambda = "Type.AnonymousLambda", TypeAnonymousParam = "Type.AnonymousParam",
    ModErased = "Mod.Erased", ModInto = "Mod.Into", ModTracked = "Mod.Tracked", DefnMacro = "Defn.Macro",
    TermDo = "Term.Do", TermSubMatch = "Term.SubMatch", LitIntXL = "Lit.IntXL", LitFloatXL = "Lit.FloatXL",
    TypeExistential = "Type.Existential", TypeFunctionArg = "Type.FunctionArg", TypeAnonymousName = "Type.AnonymousName",
    TypeAssign = "Type.Assign", TypeMacro = "Type.Macro", TypeBlock = "Type.Block",
);

/// The classes that extend scalameta 4.17.4's `Tree.Block` (trees2/shared/src/main/scala/scala/meta/Trees.scala: the
/// brace- or indent-delimited containers, `Tree.CasesBlock`'s with them), which `atPosWithBody` tells apart
/// (`body.is[Tree.Block]`).
const TREE_BLOCK: &[&str] = &["Stat.Block", "Term.CasesBlock", "Term.EnumeratorsBlock", "Term.Block", "Term.PartialFunction", "Type.CasesBlock", "Type.Block", "Pkg.Body", "Ctor.Block", "Template.Body", "Source"];

impl K {
    /// A node of a class that extends `Tree.Block` (`TREE_BLOCK`).
    pub fn tree_block(self) -> bool {
        TREE_BLOCK.contains(&self.name())
    }
}

names!(F, FIELD_NAMES:
    Stats = "stats", Ref = "ref", Importers = "importers", Importees = "importees", Name = "name", Rename = "rename",
    Tpe = "tpe", Mods = "mods", Pats = "pats", Decltpe = "decltpe", Rhs = "rhs",
    Body = "body", Templ = "templ", Ctor = "ctor", Inits = "inits", SelfDecl = "self",
    Values = "values", Default = "default", Init = "init", Qual = "qual", Fun = "fun", Args = "args",
    Lhs = "lhs", Op = "op", Arg = "arg", Cond = "cond", Thenp = "thenp", Elsep = "elsep",
    Expr = "expr", Cases = "cases", Catchp = "catchp", Finallyp = "finallyp", Enums = "enums",
    Prefix = "prefix", Pat = "pat", Lo = "lo", Hi = "hi", Annots = "annots",
    ArgClause = "argClause", ArgClauses = "argClauses", Mod = "mod", CasesBlock = "casesBlock", EnumsBlock = "enumsBlock",
    CatchClause = "catchClause", ParamClause = "paramClause", ParamClauseGroups = "paramClauseGroups",
    ParamClauseGroup = "paramClauseGroup", TparamClause = "tparamClause", ParamClauses = "paramClauses",
    TargClause = "targClause", Bounds = "bounds", Context = "context", Res = "res", Typ = "typ", Derives = "derives",
    Within = "within", Parts = "parts", Thisp = "thisp", Superp = "superp", Variant = "variant",
    EmptySelf = "emptySelf", View = "view", EarlyClause = "earlyClause", TypeDefs = "typeDefs",
);

/// A node: its kind, its span, its parent, its children (`Tree::entries[first..first + count]`, each with its
/// field, in source order), its text where it has one (a name's, a literal's value as written), its flags and the
/// comments scalameta's parser gives it (`begComment`, `endComment`), each from its first comment's start to its last
/// one's end.
#[derive(Clone, Copy, Debug)]
pub struct Node {
    pub kind: K,
    pub span: Span,
    pub parent: u32,
    pub first: u32,
    pub count: u32,
    pub value: u32,
    pub flags: u32,
    pub beg_comment: Option<Span>,
    pub end_comment: Option<Span>,
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
    pub fn children(&self, n: u32) -> &[(F, u32)] {
        let node = &self.nodes[n as usize];
        &self.entries[node.first as usize..(node.first + node.count) as usize]
    }

    /// The children of the tree scalameta's has: a template's empty `Self` (`F::EmptySelf`), which `Template.self` answers
    /// where none is written, is none of them.
    pub fn written_children(&self, n: u32) -> impl DoubleEndedIterator<Item = (F, u32)> + '_ {
        self.children(n).iter().copied().filter(|&(f, _)| f != F::EmptySelf)
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
            for (_, c) in self.written_children(n).rev() {
                stack.push((c, depth + 1));
            }
        }
        out
    }
}

pub fn is_importee(k: K) -> bool {
    matches!(k, K::ImporteeName | K::ImporteeRename | K::ImporteeUnimport | K::ImporteeWildcard | K::ImporteeGiven | K::ImporteeGivenAll)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The port's tree of `text` under `dialect` over the pieces teq's lexer kept, or why there is none: the pieces
    /// missing, or scalameta's refusal.
    fn parsed(text: &str, dialect: super::super::parse::Dialect) -> Result<Tree, String> {
        let interner = crate::intern::Interner::new();
        let (asts, _) = crate::frontend::parse_one(text, &interner, crate::parser::Syntax { keep: true, index: true, ..Default::default() });
        let kept = asts.first().and_then(|a| a.notes.source.as_deref()).ok_or("no source kept")?;
        let pieces = super::super::source::editable(text, kept).map_err(|at| format!("no token stream past {}", at))?;
        super::super::parse::parse(text, &pieces, dialect).map_err(|s| match s {
            super::super::parse::Stop::Refused(_, message) => message,
            super::super::parse::Stop::NotYet(p) => format!("`{}` not ported", p),
        })
    }

    fn tree(text: &str) -> Tree {
        parsed(text, super::super::parse::Dialect::SCALA3).unwrap()
    }

    /// The tree of `text` as support/DumpLayout.scala prints the rules' tree (and support/MetaDump.scala scalameta's): a
    /// node a line, its kind, its start and its end in UTF-16 units, its children indented; or the refusal.
    fn layout(text: &str) -> Result<String, String> {
        parsed(text, super::super::parse::Dialect::SCALA3).map(|t| layout_of(&t, text))
    }

    /// `layout` is DumpLayout's view: over every fixture of tests/fix that DumpLayout runs on, the tree a typed run
    /// printed (its expected.txt) is the one `layout` prints of the parse alone.
    #[test]
    fn layout_is_dump_layouts() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fix");
        let mut checked = 0;
        let mut differ = Vec::new();
        for rule in std::fs::read_dir(&root).unwrap().flatten() {
            for case in std::fs::read_dir(rule.path()).into_iter().flatten().flatten() {
                let (input, expected) = (case.path().join("input.scala"), case.path().join("expected.txt"));
                let (Ok(text), Ok(lints)) = (std::fs::read_to_string(&input), std::fs::read_to_string(&expected)) else { continue };
                if !text.contains("support/DumpLayout.scala") || text.lines().any(|l| l.starts_with("// teq: ")) {
                    continue;
                }
                let printed: String = lints.lines().skip_while(|l| !l.ends_with("BEGIN")).skip(1).take_while(|l| *l != "END").map(|l| format!("{}\n", l)).collect();
                checked += 1;
                if layout(&text).as_deref() != Ok(printed.as_str()) {
                    differ.push(case.path().display().to_string());
                }
            }
        }
        assert!(checked >= 10, "only {} DumpLayout fixtures", checked);
        assert!(differ.is_empty(), "layout differs from DumpLayout's at {:?}", differ);
    }

    /// The scanner's stream as scalameta's (`scanner::region_line`): over every program of the corpus, the tokens the scanner
    /// makes in the order its parser reads them and the indented regions each stands in, as support/RegionStream.scala
    /// wrote them (`#!regions`), by their starts in UTF-16 units: the lexer's tokens alone, whatever the parser makes of
    /// them. Left out: a program scalameta's tokenizer refuses and one whose tokens teq's lexer does not keep whole.
    /// `TEQ_REGION_DIFFS=<file>` writes every difference there.
    #[test]
    fn scalameta_regions() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fix/scalameta");
        let mut files: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "txt") && p.file_name().is_some_and(|n| n != "LISTED.txt")).collect();
        files.sort();
        let (mut programs, mut compared, mut refused) = (0, 0, 0);
        let mut differ: Vec<String> = Vec::new();
        for f in files {
            let suite = f.file_stem().unwrap().to_string_lossy().into_owned();
            let text = std::fs::read_to_string(&f).unwrap();
            for entry in text.split("\n#!program ").skip(1) {
                let (header, rest) = entry.split_once('\n').unwrap();
                let (program, rest) = rest.split_once("#!scalac ").unwrap();
                let program = program.strip_suffix("\n#!no-final-newline\n").unwrap_or(program);
                let expected = rest.lines().find_map(|l| l.strip_prefix("#!regions ")).unwrap_or_else(|| panic!("{} {}: no #!regions line", suite, header));
                programs += 1;
                let interner = crate::intern::Interner::new();
                let (asts, _) = crate::frontend::parse_one(program, &interner, crate::parser::Syntax { keep: true, index: true, ..Default::default() });
                let pieces = asts.first().and_then(|a| a.notes.source.as_deref()).and_then(|kept| super::super::source::editable(program, kept).ok());
                let (Some(pieces), true) = (pieces, expected != "refused") else {
                    refused += 1;
                    continue;
                };
                let units = crate::fix::host::Utf16::of(program);
                let got = super::super::scanner::region_line(program, &pieces, |b| units.units(b));
                if got == expected {
                    compared += 1;
                } else {
                    differ.push(format!("{} {}\n{}\n--- scalameta\n{}\n--- teq\n{}", suite, header, program, expected, got));
                }
            }
        }
        if let Some(out) = std::env::var_os("TEQ_REGION_DIFFS") {
            std::fs::write(&out, differ.join("\n=====\n")).unwrap();
        }
        eprintln!("scalameta regions: {} programs, {} as scalameta's, {} refused, {} differ", programs, compared, refused, differ.len());
        assert!(programs > 1000, "the corpus holds {} programs", programs);
        assert!(differ.is_empty(), "{} programs' streams differ from scalameta's; the first:\n{}", differ.len(), differ.first().map_or("", |d| d.as_str()));
    }

    /// `t` as `layout` prints a tree.
    fn layout_of(t: &Tree, text: &str) -> String {
        let units = crate::fix::host::Utf16::of(text);
        let mut out = String::new();
        let mut stack = vec![((t.nodes.len() - 1) as u32, 0usize)];
        while let Some((n, depth)) = stack.pop() {
            let node = t.nodes[n as usize];
            out.push_str(&format!("{}{} {} {}\n", "  ".repeat(depth), node.kind.name(), units.units(node.span.start), units.units(node.span.end)));
            for (_, c) in t.written_children(n).rev() {
                stack.push((c, depth + 1));
            }
        }
        out
    }

    /// What the parser makes of a program whose scalameta layout is `expected`.
    enum Ported {
        Read,
        Refused,
        NotYet(&'static str),
        Unscanned,
        Differ(String),
    }

    fn port(program: &str, expected: &str, dialect: super::super::parse::Dialect) -> Ported {
        let interner = crate::intern::Interner::new();
        let (asts, _) = crate::frontend::parse_one(program, &interner, crate::parser::Syntax { keep: true, index: true, ..Default::default() });
        let Some(pieces) = asts.first().and_then(|a| a.notes.source.as_deref()).and_then(|kept| super::super::source::editable(program, kept).ok()) else {
            return Ported::Unscanned;
        };
        match super::super::parse::parse(program, &pieces, dialect) {
            Err(super::super::parse::Stop::NotYet(p)) => Ported::NotYet(p),
            Err(super::super::parse::Stop::Refused(_, message)) => {
                if expected.strip_prefix("refused: ").is_some_and(|e| e.trim_end() == message.lines().next().unwrap_or("")) {
                    Ported::Refused
                } else {
                    Ported::Differ(format!("--- scalameta\n{}--- port\nrefused: {}\n", expected, message))
                }
            }
            Ok(t) => {
                let shown = layout_of(&t, program);
                if shown == expected { Ported::Read } else { Ported::Differ(format!("--- scalameta\n{}--- port\n{}", expected, shown)) }
            }
        }
    }

    /// The node contract whole: every program of scalameta 4.17.4's own parser tests for the Scala 3 dialect
    /// (tests/fix/scalameta, made by support/ScalametaCorpus.scala, and the passes' probes, Probes.txt) read by the
    /// parser (`parse::parse`) as scalameta reads it, structure and spans through `layout`, or refused with the corpus's
    /// message, none stopping at a production. Counted apart: a program scalac's parser refuses and one
    /// tests/fix/scalameta/LISTED.txt lists with its reason, where the parser's reading differs; a listed one read as
    /// scalameta's leaves the list. `TEQ_PARSE_SAMPLES=<dir>` adds every `<name>.scala` there beside its MetaDump
    /// layout (`<name>.scala.meta`), read under `dialects.Scala3` or `TEQ_PARSE_DIALECT=Scala213Source3`'s;
    /// `TEQ_PARSE_DIFFS=<file>` writes every difference there, those counted apart marked so.
    #[test]
    fn scalameta_parse() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fix/scalameta");
        let mut listed = std::collections::HashSet::new();
        for line in std::fs::read_to_string(dir.join("LISTED.txt")).unwrap().lines().filter(|l| !l.starts_with('#')) {
            let mut words = line.splitn(3, ' ');
            let (suite, n, reason) = (words.next().unwrap(), words.next().unwrap(), words.next().unwrap());
            if let Some(pin) = reason.strip_prefix("pinned as ") {
                assert!(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(pin).is_file(), "{} {}: no pin {}", suite, n, pin);
            }
            listed.insert(format!("{} {}", suite, n));
        }
        let mut files: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "txt") && p.file_name().is_some_and(|n| n != "LISTED.txt")).collect();
        files.sort();
        // A program's name, its key, its text, scalameta's layout of it, whether scalac refuses it.
        let mut programs: Vec<(String, String, String, String, bool)> = Vec::new();
        for f in files {
            let suite = f.file_stem().unwrap().to_string_lossy().into_owned();
            let text = std::fs::read_to_string(&f).unwrap();
            for entry in text.split("\n#!program ").skip(1) {
                let (header, rest) = entry.split_once('\n').unwrap();
                let (program, rest) = rest.split_once("#!scalac ").unwrap();
                let program = program.strip_suffix("\n#!no-final-newline\n").unwrap_or(program);
                let expected = format!("{}\n", rest.split_once("#!layout\n").unwrap().1.trim_end_matches('\n'));
                let n = header.split_whitespace().next().unwrap();
                programs.push((format!("{} {}", suite, header), format!("{} {}", suite, n), program.to_string(), expected, rest.starts_with("refuses: ")));
            }
        }
        let corpus = programs.len();
        let samples_dialect = match std::env::var("TEQ_PARSE_DIALECT").as_deref() {
            Ok("Scala213Source3") => super::super::parse::Dialect::SCALA213_SOURCE3,
            _ => super::super::parse::Dialect::SCALA3,
        };
        if let Some(samples) = std::env::var_os("TEQ_PARSE_SAMPLES") {
            let mut found: Vec<_> = std::fs::read_dir(&samples).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "scala")).collect();
            found.sort();
            for f in found {
                let Ok(expected) = std::fs::read_to_string(f.with_extension("scala.meta")) else { continue };
                programs.push((f.display().to_string(), String::new(), std::fs::read_to_string(&f).unwrap(), expected, false));
            }
        }
        let (mut read, mut refused, mut unscanned, mut scalac_refuses, mut listed_apart) = ([0, 0], [0, 0], [0, 0], [0, 0], [0, 0]);
        let mut not_yet: std::collections::BTreeMap<&'static str, usize> = Default::default();
        let mut differ: Vec<String> = Vec::new();
        let mut apart: Vec<String> = Vec::new();
        let mut matched_listed: Vec<String> = Vec::new();
        for (i, (name, key, program, expected, by_scalac)) in programs.iter().enumerate() {
            let set = usize::from(i >= corpus);
            let is_listed = listed.contains(key);
            match port(program, expected, if set == 0 { super::super::parse::Dialect::SCALA3 } else { samples_dialect }) {
                Ported::NotYet(p) => *not_yet.entry(p).or_default() += 1,
                Ported::Unscanned => unscanned[set] += 1,
                Ported::Refused | Ported::Read if is_listed => matched_listed.push(key.clone()),
                Ported::Refused => refused[set] += 1,
                Ported::Read => read[set] += 1,
                Ported::Differ(d) if *by_scalac => {
                    scalac_refuses[set] += 1;
                    apart.push(format!("{} (scalac refuses)\n{}\n{}", name, program, d));
                }
                Ported::Differ(d) if is_listed => {
                    listed_apart[set] += 1;
                    apart.push(format!("{} (listed)\n{}\n{}", name, program, d));
                }
                Ported::Differ(d) => differ.push(format!("{}\n{}\n{}", name, program, d)),
            }
        }
        if let Some(out) = std::env::var_os("TEQ_PARSE_DIFFS") {
            std::fs::write(&out, differ.iter().chain(&apart).cloned().collect::<Vec<_>>().join("\n=====\n")).unwrap();
        }
        for (set, what, count) in [(0, "corpus", corpus), (1, "samples", programs.len() - corpus)] {
            if count > 0 {
                eprintln!(
                    "scalameta parse, {}: {} programs, {} read as scalameta's, {} refused as scalameta refuses, {} scalac refuses, {} listed, {} not scanned",
                    what, count, read[set], refused[set], scalac_refuses[set], listed_apart[set], unscanned[set]
                );
            }
        }
        eprintln!("not read, by production: {:?}; {} differ", not_yet, differ.len());
        assert!(corpus > 1000, "the corpus holds {} programs", corpus);
        assert!(not_yet.is_empty(), "programs stop at productions not ported: {:?}", not_yet);
        assert!(matched_listed.is_empty(), "listed but read as scalameta's now: {:?}", matched_listed);
        assert!(differ.is_empty(), "{} programs differ from scalameta's; the first:\n{}", differ.len(), differ.first().map_or("", |d| d.as_str()));
    }

    /// The tree as text: each node's kind, its text or its value, its children indented.
    fn show(t: &Tree, text: &str, n: u32, depth: usize, out: &mut String) {
        let node = t.nodes[n as usize];
        let s = &text[node.span.start as usize..node.span.end as usize];
        out.push_str(&format!("{}{} `{}`\n", "  ".repeat(depth), node.kind.name(), s.replace('\n', "⏎")));
        for (_, c) in t.written_children(n) {
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
    Pkg.Body `import cats.implicits._⏎object O {⏎  final val x = List(1).map(_ => ())⏎  def f(a: Int = 1)(implicit b: Int): Unit = a match { case Nil => () }⏎}`
      Import `import cats.implicits._`
        Importer `cats.implicits._`
          Term.Select `cats.implicits`
            Term.Name `cats`
            Term.Name `implicits`
          Importee.Wildcard `_`
      Defn.Object `object O {⏎  final val x = List(1).map(_ => ())⏎  def f(a: Int = 1)(implicit b: Int): Unit = a match { case Nil => () }⏎}`
        Term.Name `O`
        Template `{⏎  final val x = List(1).map(_ => ())⏎  def f(a: Int = 1)(implicit b: Int): Unit = a match { case Nil => () }⏎}`
          Template.Body `{⏎  final val x = List(1).map(_ => ())⏎  def f(a: Int = 1)(implicit b: Int): Unit = a match { case Nil => () }⏎}`
            Defn.Val `final val x = List(1).map(_ => ())`
              Mod.Final `final`
              Pat.Var `x`
                Term.Name `x`
              Term.Apply `List(1).map(_ => ())`
                Term.Select `List(1).map`
                  Term.Apply `List(1)`
                    Term.Name `List`
                    Term.ArgClause `(1)`
                      Lit.Int `1`
                  Term.Name `map`
                Term.ArgClause `(_ => ())`
                  Term.Function `_ => ()`
                    Term.ParamClause `_`
                      Term.Param `_`
                        Name.Placeholder `_`
                    Lit.Unit `()`
            Defn.Def `def f(a: Int = 1)(implicit b: Int): Unit = a match { case Nil => () }`
              Term.Name `f`
              Member.ParamClauseGroup `(a: Int = 1)(implicit b: Int)`
                Type.ParamClause ``
                Term.ParamClause `(a: Int = 1)`
                  Term.Param `a: Int = 1`
                    Term.Name `a`
                    Type.Name `Int`
                    Lit.Int `1`
                Term.ParamClause `(implicit b: Int)`
                  Term.Param `b: Int`
                    Mod.Implicit `implicit`
                    Term.Name `b`
                    Type.Name `Int`
                  Mod.Implicit `implicit`
              Type.Name `Unit`
              Term.Match `a match { case Nil => () }`
                Term.Name `a`
                Term.CasesBlock `{ case Nil => () }`
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
            let Ok(t) = parsed(&text, super::super::parse::Dialect::SCALA3) else { continue };
            let root_id = (t.nodes.len() - 1) as u32;
            for (i, n) in t.nodes.iter().enumerate() {
                assert!(n.span.start <= n.span.end && n.span.end as usize <= text.len(), "{}: node {} {} {:?}", f.display(), i, n.kind.name(), n.span);
                if i as u32 != root_id {
                    assert!(n.parent != NONE, "{}: node {} {} {:?} has no parent", f.display(), i, n.kind.name(), n.span);
                }
                // In source order but where scalameta's fields are not: a clause's modifier after its values (an `inline
                // if`'s and an inline match's too), a type definition's bounds after its body, a template's `derives`
                // after its body, an interpolation's arguments after its parts, a bounds alias's name before its bound,
                // and an empty node, where scalameta's parser stands.
                let kids: Vec<(F, u32)> = t
                    .children(i as u32)
                    .iter()
                    .copied()
                    .filter(|&(f, c)| {
                        let moved = f == F::Mod
                            || f == F::Derives
                            || (n.kind == K::DefnType && f == F::Bounds)
                            || (matches!(n.kind, K::TermIf | K::TermMatch) && f == F::Mods)
                            || (matches!(n.kind, K::TermInterpolate | K::PatInterpolate) && f == F::Args)
                            || (n.kind == K::TypeBoundsAlias && f == F::Name);
                        !moved && t.nodes[c as usize].span.end > t.nodes[c as usize].span.start
                    })
                    .collect();
                for w in kids.windows(2) {
                    assert!(t.nodes[w[0].1 as usize].span.start <= t.nodes[w[1].1 as usize].span.start, "{}: children of {} out of order", f.display(), n.kind.name());
                }
                // A node ends at or past each child of its own tokens.
                for &(_, c) in t.children(i as u32) {
                    let child = t.nodes[c as usize];
                    assert!(child.span.end <= child.span.start || child.span.end <= n.span.end, "{}: {} {:?} ends before its {} {:?}", f.display(), n.kind.name(), n.span, child.kind.name(), child.span);
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

/// The `java.math.BigDecimal` scalameta's `AnyDecimal.toBigDecimalOpt` makes of the unsigned decimal literal `written`
/// (its underscores left out), as its `toString` writes it (scalameta's `format` of a `Lit.Double` or a `Lit.Float`),
/// and whether it is zero: the digits as written, their trailing zeros stripped where the scale is over 1 or an
/// exponent moved the point (`movePointRight`, its scale at least 0), `1e3` `1E+3`, `.5` `0.5`, `0.00` `0`.
pub(crate) fn big_decimal_text(written: &str) -> (String, bool) {
    let text: String = written.chars().filter(|&c| c != '_').collect();
    let (mantissa, exponent) = match text.find(['e', 'E']) {
        Some(k) => (&text[..k], text[k + 1..].parse::<i64>().unwrap_or(0)),
        None => (&text[..], 0),
    };
    let (int, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let mut digits: String = format!("{int}{frac}").trim_start_matches('0').to_string();
    if digits.is_empty() {
        digits.push('0');
    }
    let mut scale = frac.len() as i64;
    let strip = |digits: &mut String, scale: &mut i64| {
        if digits == "0" {
            *scale = 0;
            return;
        }
        while digits.len() > 1 && digits.ends_with('0') {
            digits.pop();
            *scale -= 1;
        }
    };
    if exponent == 0 {
        if scale > 1 {
            strip(&mut digits, &mut scale);
        }
    } else {
        scale -= exponent;
        strip(&mut digits, &mut scale);
    }
    let zero = digits == "0";
    let n = digits.len() as i64;
    let adjusted = -scale + (n - 1);
    let text = if scale == 0 {
        digits
    } else if scale > 0 && adjusted >= -6 {
        if n > scale {
            format!("{}.{}", &digits[..(n - scale) as usize], &digits[(n - scale) as usize..])
        } else {
            format!("0.{}{}", "0".repeat((scale - n) as usize), digits)
        }
    } else {
        let rest = if n > 1 { format!(".{}", &digits[1..]) } else { String::new() };
        format!("{}{}E{}{}", &digits[..1], rest, if adjusted >= 0 { "+" } else { "" }, adjusted)
    };
    (text, zero)
}

/// An interpolation part's value from its text: `$$` and `$"` the character after the `$`; with `unicode`, an
/// unescaped backslash's `\uXXXX` (its `u`s one or more) the UTF-16 unit, a pair of them one character, a lone
/// surrogate kept as teq's strings keep it (`text::from_units`).
pub(crate) fn part_value(text: &str, unicode: bool) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut units: Vec<u16> = Vec::with_capacity(chars.len());
    let mut backslashes = 0usize;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '$' && matches!(chars.get(i + 1), Some('$') | Some('"')) {
            units.push(chars[i + 1] as u16);
            i += 2;
            backslashes = 0;
            continue;
        }
        if unicode && c == '\\' && backslashes % 2 == 0 && chars.get(i + 1) == Some(&'u') {
            let mut j = i + 1;
            while chars.get(j) == Some(&'u') {
                j += 1;
            }
            let hex: String = chars.get(j..j + 4).map(|h| h.iter().collect()).unwrap_or_default();
            if let Ok(unit) = u16::from_str_radix(&hex, 16) {
                units.push(unit);
                i = j + 4;
                backslashes = 0;
                continue;
            }
        }
        backslashes = if c == '\\' { backslashes + 1 } else { 0 };
        let mut buf = [0u16; 2];
        units.extend_from_slice(c.encode_utf16(&mut buf));
        i += 1;
    }
    crate::text::from_units(units)
}
