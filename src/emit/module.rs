//! ES module output. A single file keeps the program in its function wrapper: import statements
//! in front of the runtime, exports behind the program, which leave the wrapper through its
//! result. The split output lays the same text out over one file per module (`layout.rs`): each
//! module imports the bindings it refers to from the modules that define them and exports what
//! the others refer to, `rt.mjs` holds the runtime, and `main.mjs` runs the entry point and
//! re-exports the `@jsExport` definitions.
//!
//! Modules of one program refer to each other in cycles, which ES modules allow as long as no
//! module reads a binding of another while the modules load. Nothing a module runs at load does:
//! a class with a superclass is written as `extends Object`, objects and top-level vals are
//! initialised on first access, and the things that do read other classes, the `$ext` calls that
//! put the superclasses in place, the `$cls` registrations and the enum values created with
//! `new`, are deferred into `$init` and `$enums`, which `main.mjs` calls once every module has
//! loaded, registrations first as in the single file.
//!
//! Under `--hot` a development server re-executes an edited module and its importers, `main.mjs`
//! among them, inside a page that has booted: `$init` and `$enums` then run once per module
//! instance, so that the modules kept from the first load keep their enum values, and every module
//! exports `$hot()`, which calls the accessors of its objects so that a re-executed module can
//! re-run their bodies.
//!
//! Which modules a swap re-executes is in the modules' imports, which is what a server's walk
//! from an edited module follows: the modules of `--module-per-file` packages import each other
//! and `main.mjs` imports them all, so an update of one travels up the per-file modules that
//! import it to `main.mjs` and what accepts above it. Every other module (a package's, the
//! std's) imports no per-file module: it takes the definitions it refers to through the runtime
//! (`$hotUse`, from what the per-file module gives with `$hotProvide`), which rebinds them to
//! the instance a swap made, and accepts an update of its own, so that nothing above it runs
//! again with it: running shared code again makes classes anew whose instances the page holds,
//! so the page has to be loaded again. No module accepts an update of another: a server keeps
//! the text of a module that does as it served it, with the address of the accepted module as
//! it was before the swap, and the next page load runs that module twice.
//!
//! Every module ends with a footer that tells the runtime that it ran, under which address and
//! with which text, by the text's hash (`$hotRan`; `$hotBooted` for `main.mjs`). A module that
//! must not run again leaves the wish to load the page again when it does, as `main.mjs` does
//! after a swap too wide to be one. `hot-build.mjs`, which imports nothing of the program and
//! which the writer publishes when every module of the build is in place, lists the hash of
//! every module (`$hotBuild`): the runtime carries a wish out once that listing has, for every
//! module the page ran, the text the page ran, which is when the build that made the wish is
//! published whole and its updates have arrived.

use super::layout::Module;
use super::names::{is_js_identifier, js_string, NO_SCOPE};
use super::runtime::{Runtime, Used};
use super::{Chunk, Def, Emitter, Modules, HASH_LEN};
use crate::intern::{FxMap, Name};
use crate::symbols::{Owner, SymKind};
use crate::types::SymId;
use std::fmt::Write;

pub struct ModuleParts {
    pub imports: String,
    /// Everything between the runtime and the wrapper function of the program.
    pub program_start: String,
    pub exports: String,
}

impl<'a> Emitter<'a> {
    /// Appends the `return` of the exported values to the program and yields the module frame.
    pub fn module_parts(&mut self) -> ModuleParts {
        let prog = self.prog;
        let mut parts =
            ModuleParts { imports: String::new(), program_start: "\n".to_string(), exports: String::new() };
        if !prog.is_module() {
            return parts;
        }
        let used: Vec<u32> = (0..prog.js_imports.len() as u32).filter(|&i| self.reach.imports[i as usize]).collect();
        parts.imports = self.js_import_statements(&used);
        parts.program_start.push_str("$sync = true;\n");
        if prog.js_exports.is_empty() {
            return parts;
        }
        let values = self.exported_values();
        self.line();
        self.out.push_str("return [");
        parts.program_start.push_str("const [");
        parts.exports.push_str("export { ");
        for (i, (value, name)) in values.iter().enumerate() {
            if i > 0 {
                self.out.push_str(", ");
                parts.program_start.push_str(", ");
                parts.exports.push_str(", ");
            }
            self.out.push_str(value);
            let _ = write!(parts.program_start, "$exp{}", i);
            let _ = write!(parts.exports, "$exp{} as {}", i, self.interner.get(*name));
        }
        self.out.push_str("];");
        parts.program_start.push_str("] = ");
        parts.exports.push_str(" };\n");
        parts
    }

    /// One import statement per binding of `Program::js_imports`, in index order.
    fn js_import_statements(&self, used: &[u32]) -> String {
        let mut out = String::new();
        for &i in used {
            let import = &self.prog.js_imports[i as usize];
            let name = self.interner.get(import.name);
            match name {
                // Uses of a default import read `.default` off the namespace (`Emitter::js_import`), as
                // Scala.js does: a module without a default export still loads.
                "*" | "default" => {
                    let _ = write!(out, "import * as $imp{} from ", i);
                }
                _ => {
                    out.push_str("import { ");
                    if is_js_identifier(name) {
                        out.push_str(name);
                    } else {
                        js_string(name, &mut out);
                    }
                    let _ = write!(out, " as $imp{} }} from ", i);
                }
            }
            js_string(self.interner.get(import.module), &mut out);
            out.push_str(";\n");
        }
        out
    }

    /// The JS expression of each `@jsExport` definition with its exported name.
    fn exported_values(&mut self) -> Vec<(String, Name)> {
        let exports = self.prog.js_exports.clone();
        exports
            .into_iter()
            .map(|(sym, name)| {
                let outer = std::mem::take(&mut self.out);
                self.emit_exported_value(sym);
                (std::mem::replace(&mut self.out, outer), name)
            })
            .collect()
    }

    fn emit_exported_value(&mut self, sym: SymId) {
        let info = self.syms.sym(sym);
        if let Some(import) = info.js_import {
            let binding = self.js_import(import);
            self.out.push_str(&binding);
            return;
        }
        // A member of an object is read from the instance, whose body runs at module load.
        if let Owner::Class(c) = info.owner {
            let accessor = self.module_accessor(c);
            let name = self.sym_name(sym);
            let _ = if info.kind == SymKind::Def {
                write!(self.out, "(...$a) => {}().{}(...$a)", accessor, name)
            } else {
                write!(self.out, "{}().{}", accessor, name)
            };
            return;
        }
        let name = self.sym_name(sym);
        if info.kind != SymKind::Def {
            self.out.push_str(&name);
            if !self.const_vals[sym.idx()] {
                self.out.push_str("()");
            }
            return;
        }
        let params = info.sig.as_ref().map_or(0, |sig| sig.clauses.iter().map(|c| c.params.len()).sum());
        let has_rest = info.sig.as_ref().map_or(false, |sig| {
            sig.clauses.iter().flat_map(|c| c.params.iter()).last().map_or(false, |p| p.repeated)
        });
        let Some(array_seq) = self.array_seq.filter(|_| has_rest) else {
            self.out.push_str(&name);
            return;
        };
        // JS callers pass the repeated parameter as rest arguments.
        let seq = self.class_name(array_seq);
        let fixed: String = (0..params - 1).map(|i| format!("$a{}, ", i)).collect();
        let _ = write!(self.out, "({0}...$rest) => {1}({0}new {2}($rest))", fixed, name, seq);
    }

    /// The names a definition goes by in the given scope; a module exports and imports them.
    fn def_names(&mut self, def: Def, scope: u32) -> Vec<String> {
        self.scope = scope;
        match def {
            Def::Class(c) => {
                let mut names = vec![self.class_name(c).to_string()];
                if self.syms.class(c).kind == crate::symbols::ClassKind::Object {
                    names.push(self.module_accessor(c));
                }
                names
            }
            Def::Sym(s) => match self.syms.sym(s).kind {
                SymKind::EnumValue(_) => vec![self.enum_value_name(s)],
                SymKind::Var => {
                    let name = self.sym_name(s);
                    vec![name.to_string(), format!("{}set", name)]
                }
                _ => vec![self.sym_name(s).to_string()],
            },
            Def::File(f) => vec![self.prog.file_init_name(f)],
            Def::Outlined(f) => vec![self.outline.funs[f as usize].name.clone()],
        }
    }

    /// Import and export lists are ordered by the names they carry, which depend on the
    /// definitions alone, not by ids.
    fn sort_by_name(&mut self, defs: &mut Vec<Def>, scope: u32) {
        let mut keyed: Vec<(String, Def)> = defs.iter().map(|&d| (self.def_names(d, scope).swap_remove(0), d)).collect();
        keyed.sort();
        *defs = keyed.into_iter().map(|(_, d)| d).collect();
    }

    /// Lays the chunks out as one file per module, with `main.mjs` first; `self` has emitted the
    /// entry point. Modules that hold nothing are left out.
    pub fn split_modules(
        mut self,
        modules: &[Module],
        chunks: Vec<Vec<Chunk>>,
        runtime: &Runtime,
        hot: bool,
        lap: &mut crate::measure::Lap,
    ) -> Modules {
        let n = modules.len();
        let main = n;
        let mut home: FxMap<Def, usize> = FxMap::default();
        for (m, cs) in chunks.iter().enumerate() {
            for c in cs {
                for &d in &c.bindings.defs {
                    home.insert(d, m);
                }
            }
        }
        // Per module, the definitions it takes from each other module, and what the others take.
        let mut imports: Vec<Vec<(usize, Vec<Def>)>> = vec![Vec::new(); n + 1];
        let mut exports: Vec<Vec<Def>> = vec![Vec::new(); n];
        let mut helpers: Vec<Used> = vec![runtime.none_used(); n + 1];
        let mut js_imports: Vec<Vec<u32>> = vec![Vec::new(); n + 1];
        self.scope = NO_SCOPE;
        let values = self.exported_values();
        let entry_bindings = std::mem::take(&mut self.bindings);
        let bindings_of = |m: usize| -> Vec<&super::Bindings> {
            if m == main {
                vec![&entry_bindings]
            } else {
                chunks[m].iter().map(|c| &c.bindings).collect()
            }
        };
        let scope_of = |m: usize| if m == main { NO_SCOPE } else { modules[m].scope };
        for m in 0..=n {
            let mut refs: Vec<Def> = Vec::new();
            for b in bindings_of(m) {
                refs.extend(b.classes.iter().map(|&c| Def::Class(c)));
                refs.extend(b.syms.iter().map(|&s| Def::Sym(s)));
                refs.extend(b.files.iter().map(|&f| Def::File(f)));
                refs.extend(b.outlined.iter().map(|&f| Def::Outlined(f)));
                js_imports[m].extend(&b.imports);
            }
            refs.sort_unstable();
            refs.dedup();
            // What the module takes from the others, by name, then grouped by the module it is
            // taken from in that order.
            refs.retain(|d| home.get(d).is_some_and(|&h| h != m));
            self.sort_by_name(&mut refs, scope_of(m));
            js_imports[m].sort_unstable();
            js_imports[m].dedup();
            let mut taken: Vec<(usize, Def)> = refs.into_iter().map(|d| (home[&d], d)).collect();
            taken.sort_by_key(|&(h, _)| h);
            for (h, d) in taken {
                match imports[m].last_mut() {
                    Some((from, defs)) if *from == h => defs.push(d),
                    _ => imports[m].push((h, vec![d])),
                }
                exports[h].push(d);
            }
            if m < n {
                for c in &chunks[m] {
                    Runtime::merge(&mut helpers[m], &c.helpers);
                }
            }
        }
        // Under --hot a module that is not per file takes what it refers to in a per-file
        // module through the runtime: per per-file module, the names it gives, and whether
        // such a module holds a class of it to what it is (`Held`: as a parent, in a test, by
        // a compared enum value), which the rebinding after a swap would break.
        let mut given: Vec<Vec<String>> = vec![Vec::new(); n];
        let mut held = vec![false; n];
        if hot {
            for m in (0..n).filter(|&m| !modules[m].per_file) {
                for (from, defs) in imports[m].clone() {
                    if !modules[from].per_file {
                        continue;
                    }
                    runtime.mark("$hotUse(", &mut helpers[m]);
                    runtime.mark("$hotProvide(", &mut helpers[from]);
                    for d in defs {
                        given[from].extend(self.def_names(d, modules[from].scope));
                    }
                }
                for &(c, _) in chunks[m].iter().flat_map(|c| &c.bindings.held) {
                    if let Some(&from) = home.get(&Def::Class(c)) {
                        held[from] |= modules[from].per_file;
                    }
                }
            }
            for names in given.iter_mut() {
                names.sort_unstable();
                names.dedup();
            }
        }
        // Under --hot a module run again drops what it registered for reflective instantiation
        // before it registers anew, so a class it no longer registers is no longer found, and
        // main.mjs run again drops what the modules gone from the build registered.
        let reflect_scoped = hot && self.reach.reflective_reached;
        if reflect_scoped {
            for h in helpers.iter_mut().take(n) {
                runtime.mark("$reflectScope(", h);
            }
            runtime.mark("$reflectKeep(", &mut helpers[main]);
        }
        runtime.mark(&self.out, &mut helpers[main]);
        for (value, _) in &values {
            runtime.mark(value, &mut helpers[main]);
        }
        let sync = "$sync = true;\n";
        let mut all_helpers = runtime.none_used();
        runtime.mark(sync, &mut all_helpers);
        for h in &helpers {
            Runtime::merge(&mut all_helpers, h);
        }

        lap.done("framing: references", n);
        let mut files: Vec<(String, String)> = Vec::new();
        let mut footers: Vec<(usize, usize)> = Vec::new();
        if hot {
            for (m, h) in helpers.iter_mut().enumerate().take(n) {
                runtime.mark("$hotRan(", h);
                if !modules[m].per_file || held[m] {
                    runtime.mark("$hotFailing(", h);
                }
            }
            runtime.mark("$hotBooted(", &mut helpers[main]);
        }
        let mut present = vec![false; n];
        let mut has_init = vec![false; n];
        let mut has_enums = vec![false; n];
        for (m, module) in modules.iter().enumerate() {
            let cs = &chunks[m];
            if cs.iter().all(|c| c.is_empty()) {
                continue;
            }
            present[m] = true;
            let body: usize = cs.iter().map(|c| c.classes.len() + c.registrations.len() + c.enum_values.len() + c.funs.len() + c.vals.len()).sum();
            let mut out = String::with_capacity(body + 4096);
            self.write_imports(&mut out, m, &imports, &js_imports, &helpers, modules, runtime, &[], hot);
            for c in cs {
                out.push_str(&c.classes);
            }
            if reflect_scoped || cs.iter().any(|c| !c.registrations.is_empty()) {
                has_init[m] = true;
                write_phase(&mut out, "$init", hot, |out| {
                    if reflect_scoped {
                        out.push_str("\n$reflectScope(");
                        js_string(&module.name, out);
                        out.push_str(");");
                    }
                    for c in cs {
                        out.push_str(&c.registrations);
                    }
                });
            }
            if !cs[0].enum_names.is_empty() {
                has_enums[m] = true;
                let _ = write!(out, "\nlet {};", cs[0].enum_names.join(", "));
                write_phase(&mut out, "$enums", hot, |out| {
                    out.push_str(&cs[0].enum_values);
                    // The values exist now: the modules that take them are given them.
                    if !given[m].is_empty() {
                        out.push_str("\n$hotProvide(");
                        js_string(&module.name, out);
                        out.push_str(");");
                    }
                });
            }
            for c in cs {
                out.push_str(&c.funs);
            }
            for c in cs {
                out.push_str(&c.vals);
            }
            let mut names: Vec<String> = Vec::new();
            if has_init[m] {
                names.push("$init".to_string());
            }
            if has_enums[m] {
                names.push("$enums".to_string());
            }
            if hot {
                out.push_str("\nfunction $hot() {");
                for accessor in cs.iter().flat_map(|c| &c.accessors) {
                    let _ = write!(out, " {}();", accessor);
                }
                out.push_str(" }");
                names.push("$hot".to_string());
            }
            exports[m].sort_unstable();
            exports[m].dedup();
            let mut exported = std::mem::take(&mut exports[m]);
            self.sort_by_name(&mut exported, module.scope);
            for &d in &exported {
                names.extend(self.def_names(d, module.scope));
            }
            if !names.is_empty() {
                let _ = write!(out, "\nexport {{ {} }};\n", names.join(", "));
            } else {
                out.push('\n');
            }
            let file_name = format!("{}.mjs", module.name);
            if !given[m].is_empty() {
                out.push_str("\n$hotProvide(");
                js_string(&module.name, &mut out);
                let _ = write!(out, ", () => ({{ {} }}));\n", given[m].join(", "));
            }
            if hot {
                // A hot swap re-executes a `--module-per-file` module: it leaves its `$hot()`
                // (the accessors of its objects, which construct them anew) for the main.mjs
                // of the swap, which runs it after it has registered the fresh classes and
                // enum values, so that a development server's runtime sees the new objects
                // it reports.
                footers.push(write_footer(&mut out, if module.per_file && !held[m] { Footer::Swapped } else { Footer::Held }));
            }
            files.push((file_name, out));
        }
        lap.done("framing: module texts", files.len());

        self.scope = NO_SCOPE;
        let mut out = String::new();
        let phases: Vec<(usize, &str)> = (0..n)
            .filter(|&m| present[m])
            .flat_map(|m| {
                let mut v = Vec::new();
                if has_init[m] {
                    v.push((m, "$init"));
                }
                if has_enums[m] {
                    v.push((m, "$enums"));
                }
                v
            })
            .collect();
        self.write_imports(&mut out, main, &imports, &js_imports, &helpers, modules, runtime, &phases, hot);
        if hot {
            // An update of a per-file module reaches main.mjs through its imports alone.
            for m in (0..n).filter(|&m| present[m] && modules[m].per_file) {
                if !phases.iter().any(|(p, _)| *p == m) && !imports[main].iter().any(|(from, _)| *from == m) {
                    let _ = write!(out, "import \"./{}.mjs\";\n", modules[m].name);
                }
            }
            let _ = write!(out, "import \"./{}\";\n", super::HOT_BUILD);
        }
        if reflect_scoped {
            out.push_str("\n$reflectKeep([");
            for (i, m) in (0..n).filter(|&m| present[m]).enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                js_string(&modules[m].name, &mut out);
            }
            out.push_str("]);");
        }
        for name in ["$init", "$enums"] {
            for &(m, phase) in phases.iter().filter(|(_, p)| *p == name) {
                let _ = write!(out, "\n{}();", phase_alias(phase, &modules[m].name));
            }
        }
        out.push_str(&self.out);
        if !values.is_empty() {
            out.push_str("\nconst ");
            for (i, (value, _)) in values.iter().enumerate() {
                let _ = write!(out, "{}$exp{} = {}", if i > 0 { ", " } else { "" }, i, value);
            }
            out.push_str(";\nexport { ");
            for (i, (_, name)) in values.iter().enumerate() {
                let _ = write!(out, "{}$exp{} as {}", if i > 0 { ", " } else { "" }, i, self.interner.get(*name));
            }
            out.push_str(" };");
        }
        out.push('\n');
        // The page has booted once main.mjs has run. Run again by a hot swap, it has what the
        // swap's per-file modules left (above): it runs that in the microtask after its own
        // body, or leaves the wish to load the page again where the swap ran more modules
        // again than a swap may (a hundred, or what the server put in `__teqSwapUpTo`), a
        // wide one taking the modules that hold the page's state with it.
        if hot {
            footers.insert(0, write_footer(&mut out, Footer::Main));
        }
        files.insert(0, ("main.mjs".to_string(), out));

        let mut rt = String::new();
        if hot {
            runtime.mark("$hotRan $hotBooted $hotBuild $hotFailing", &mut all_helpers);
        }
        runtime.write(&all_helpers, &mut rt);
        rt.push_str(sync);
        let _ = write!(rt, "export {{ {} }};\n", runtime.names(&all_helpers).join(", "));
        if hot {
            footers.push(write_footer(&mut rt, Footer::Held));
        }
        files.push(("rt.mjs".to_string(), rt));
        if hot {
            let mut refresh = format!("import {{ $hotRan, $hotFailing }} from \"./rt.mjs\";\n{}", super::HOT_REFRESH);
            footers.push(write_footer(&mut refresh, Footer::Held));
            files.push(("hot-refresh.mjs".to_string(), refresh));
        }
        Modules { files, footers }
    }

    /// The import statements of module `m`: JS packages, runtime helpers, then the other modules
    /// in their order; `main.mjs` also takes the `$init` and `$enums` phases of the modules. A
    /// definition whose bare name is taken in `m` is imported under its qualified name. Under
    /// `hot` a module that is not per file has, in the place of its imports from a per-file
    /// module, bindings of its own that the runtime sets from what that module gives, again
    /// after every swap of it.
    #[allow(clippy::too_many_arguments)]
    fn write_imports(
        &mut self,
        out: &mut String,
        m: usize,
        imports: &[Vec<(usize, Vec<Def>)>],
        js_imports: &[Vec<u32>],
        helpers: &[Used],
        modules: &[Module],
        runtime: &Runtime,
        phases: &[(usize, &str)],
        hot: bool,
    ) {
        let takes = hot && m < modules.len() && !modules[m].per_file;
        let mut taken = String::new();
        out.push_str(&self.js_import_statements(&js_imports[m]));
        let helper_names = runtime.names(&helpers[m]);
        if !helper_names.is_empty() {
            let _ = write!(out, "import {{ {} }} from \"./rt.mjs\";\n", helper_names.join(", "));
        }
        // Both lists are in module order; one statement per module named in either.
        let (mut next_phase, mut next_import) = (0, 0);
        loop {
            let from_phase = phases.get(next_phase).map(|(p, _)| *p);
            let from_import = imports[m].get(next_import).map(|(h, _)| *h);
            let Some(from) = from_phase.into_iter().chain(from_import).min() else { break };
            let mut names: Vec<String> = Vec::new();
            while let Some((p, phase)) = phases.get(next_phase).filter(|(p, _)| *p == from) {
                names.push(format!("{} as {}", phase, phase_alias(phase, &modules[*p].name)));
                next_phase += 1;
            }
            if let Some((_, defs)) = imports[m].get(next_import).filter(|(h, _)| *h == from) {
                let scope = if m < modules.len() { modules[m].scope } else { NO_SCOPE };
                let mut sets = String::new();
                for &d in defs {
                    let exported = self.def_names(d, modules[from].scope);
                    let local = self.def_names(d, scope);
                    for (e, l) in exported.into_iter().zip(local) {
                        let _ = write!(sets, " {} = $m.{};", l, e);
                        names.push(if e == l || (takes && modules[from].per_file) { l } else { format!("{} as {}", e, l) });
                    }
                }
                next_import += 1;
                if takes && modules[from].per_file {
                    let _ = write!(taken, "let {};\n$hotUse(", names.join(", "));
                    js_string(&modules[from].name, &mut taken);
                    let _ = write!(taken, ", ($m) => {{{} }});\n", sets);
                    continue;
                }
            }
            let _ = write!(out, "import {{ {} }} from \"./{}.mjs\";\n", names.join(", "), modules[from].name);
        }
        out.push_str(&taken);
    }
}

/// What a module's footer under `--hot` tells the runtime besides the module's address and the
/// hash of its text.
enum Footer {
    /// A per-file module, which a swap runs again: its `$hot` is left for the swap's main.mjs.
    Swapped,
    /// A module that must not run again: it accepts an update of its own, so that nothing
    /// above it runs with it, told of one that fails, and leaves the wish to load the page
    /// again.
    Held,
    /// main.mjs: the page has booted, or a swap has run.
    Main,
}

/// Appends the footer and answers where it begins and where in the text its hash goes, which
/// the writer fills in, with the id of the build `ID_AFTER_HASH` further on.
fn write_footer(out: &mut String, footer: Footer) -> (usize, usize) {
    let body = out.len();
    out.push_str(match footer {
        Footer::Swapped | Footer::Main => "\nif (import.meta.hot) ",
        Footer::Held => "\nif (import.meta.hot) { import.meta.hot.accept($hotFailing(import.meta.url)); ",
    });
    out.push_str(if matches!(footer, Footer::Main) { "$hotBooted(import.meta.url, \"" } else { "$hotRan(import.meta.url, \"" });
    let at = out.len();
    out.extend(std::iter::repeat('0').take(HASH_LEN));
    out.push_str("\", \"");
    debug_assert_eq!(out.len(), at + super::ID_AFTER_HASH);
    out.extend(std::iter::repeat('0').take(HASH_LEN));
    out.push_str(match footer {
        Footer::Swapped => "\", $hot);\n",
        Footer::Held => "\"); }\n",
        Footer::Main => "\");\n",
    });
    (body, at)
}

/// `hot-build.mjs`: the id of the build and its modules by name, each with the hash of its
/// text and the id of the build that wrote it, in the order of the names.
pub fn hot_build(id: &str, modules: &[(&str, &str, &str)]) -> String {
    let mut out = String::from("import { $hotBuild } from \"./rt.mjs\";\nif (import.meta.hot) {\n  import.meta.hot.accept();\n  $hotBuild(import.meta.url, ");
    js_string(id, &mut out);
    out.push_str(", {\n");
    for (name, hash, writer) in modules {
        out.push_str("    ");
        js_string(name, &mut out);
        out.push_str(": [");
        js_string(hash, &mut out);
        out.push_str(", ");
        js_string(writer, &mut out);
        out.push_str("],\n");
    }
    out.push_str("  });\n}\n");
    out
}

/// The name `main.mjs` imports a module's phase under: `$init$a$b` for module `a.b`.
fn phase_alias(phase: &str, module: &str) -> String {
    format!("{}${}", phase, super::names::sanitize(&module.replace('.', "$")))
}

/// `function $init() { ... }` or `$enums`; under `hot` the body runs on the first call alone.
fn write_phase(out: &mut String, name: &str, hot: bool, body: impl FnOnce(&mut String)) {
    if hot {
        let _ = write!(out, "\nlet {}Done = false;", name);
    }
    let _ = write!(out, "\nfunction {}() {{", name);
    if hot {
        let _ = write!(out, "\n  if ({0}Done) return;\n  {0}Done = true;", name);
    }
    body(out);
    out.push_str("\n}");
}
