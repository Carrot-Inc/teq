use super::Worker;
use crate::symbols::*;
use crate::types::*;

impl<'a> Worker<'a> {
    pub fn show(&mut self, t: TypeId) -> String {
        let t = self.zonk(t);
        let mut out = String::new();
        self.show_into(t, &mut out, false);
        out
    }

    /// An open variable without a name: `?` and its place in the typing that made it
    /// (`TVarInfo::shown`), which is the same whichever worker typed it; its index where none is
    /// kept.
    fn show_var(&self, v: TVarId, out: &mut String) {
        out.push('?');
        match self.tvars[v].shown() {
            0 => out.push_str(&v.index().to_string()),
            n => out.push_str(&(n - 1).to_string()),
        }
    }

    fn show_args(&self, args: TList, out: &mut String) {
        let items = self.types.items(args);
        if items.is_empty() {
            return;
        }
        out.push('[');
        for (i, &a) in items.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            self.show_into(a, out, false);
        }
        out.push(']');
    }

    /// A type deeper than any a program writes is cut short: a cycle through a path's
    /// underlying type would otherwise never end.
    pub(super) fn show_into(&self, t: TypeId, out: &mut String, nested: bool) {
        let depth = self.show_depth.get();
        if depth > 64 {
            out.push_str("...");
            return;
        }
        self.show_depth.set(depth + 1);
        self.show_into_in(t, out, nested);
        self.show_depth.set(depth);
    }

    fn show_into_in(&self, t: TypeId, out: &mut String, nested: bool) {
        match self.types.get(t) {
            Type::Any => out.push_str("Any"),
            Type::Nothing => out.push_str("Nothing"),
            Type::Error => out.push_str("<error>"),
            Type::Wild => out.push('?'),
            Type::BoundedWild(lo, hi) => {
                out.push('?');
                if lo != NOTHING {
                    out.push_str(" >: ");
                    self.show_into(lo, out, true);
                }
                if hi != ANY {
                    out.push_str(" <: ");
                    self.show_into(hi, out, true);
                }
            }
            Type::Class(c, args) => {
                let items = self.types.items(args);
                if self.is_context_function_class(c) {
                    let arity = items.len() - 1;
                    if nested {
                        out.push('(');
                    }
                    out.push('(');
                    for (i, &a) in items[..arity].iter().enumerate() {
                        if i > 0 {
                            out.push_str(", ");
                        }
                        self.show_into(a, out, false);
                    }
                    out.push_str(") ?=> ");
                    self.show_into(items[arity], out, false);
                    if nested {
                        out.push(')');
                    }
                } else if self.is_function_class(c) {
                    let arity = items.len() - 1;
                    if nested {
                        out.push('(');
                    }
                    if arity == 1 && !self.is_function_type(items[0]) && self.b.by_name.map_or(true, |b| !matches!(self.types.get(items[0]), Type::Class(k, _) if k == b)) {
                        self.show_into(items[0], out, true);
                    } else {
                        out.push('(');
                        for (i, &a) in items[..arity].iter().enumerate() {
                            if i > 0 {
                                out.push_str(", ");
                            }
                            self.show_into(a, out, false);
                        }
                        out.push(')');
                    }
                    out.push_str(" => ");
                    self.show_into(items[arity], out, false);
                    if nested {
                        out.push(')');
                    }
                } else if self.is_tuple_class(c) && !items.is_empty() {
                    out.push('(');
                    for (i, &a) in items.iter().enumerate() {
                        if i > 0 {
                            out.push_str(", ");
                        }
                        self.show_into(a, out, false);
                    }
                    out.push(')');
                } else if Some(c) == self.b.by_name {
                    out.push_str("=> ");
                    self.show_into(items[0], out, true);
                } else if Some(c) == self.b.named_tuple && self.show_named_tuple(&items, out) {
                } else {
                    let info = self.syms.class(c);
                    // An object's copy of a trait's opaque type is named by its prefix, as scalac's
                    // `Html.Tag` is; the copy is not among the object's nested classes.
                    if let (ClassKind::Opaque, Owner::Class(o)) = (info.kind, info.owner) {
                        if self.syms.class(o).nested.get(&info.name) != Some(&c) {
                            out.push_str(self.interner.get(self.syms.class(o).name));
                            out.push('.');
                        }
                    }
                    out.push_str(self.interner.get(info.name));
                    self.show_args(args, out);
                }
            }
            Type::Param(p) => out.push_str(self.interner.get(self.syms.tparam(p).name)),
            Type::AppParam(p, args) => {
                out.push_str(self.interner.get(self.syms.tparam(p).name));
                self.show_args(args, out);
            }
            Type::Ctor(c) => out.push_str(self.interner.get(self.syms.class(c).name)),
            Type::Lambda(ps, body) => {
                self.show_args(ps, out);
                out.push_str(" =>> ");
                self.show_into(body, out, false);
            }
            Type::Poly(ps, body) => {
                let params = self.types.poly_params(ps);
                let bounds = self.types.poly_bounds(ps);
                out.push('[');
                for (i, &p) in params.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    self.show_into(p, out, false);
                    let (lo, hi) = (bounds.get(2 * i).copied().unwrap_or(NOTHING), bounds.get(2 * i + 1).copied().unwrap_or(ANY));
                    if lo != NOTHING {
                        out.push_str(" >: ");
                        self.show_into(lo, out, false);
                    }
                    if hi != ANY {
                        out.push_str(" <: ");
                        self.show_into(hi, out, false);
                    }
                }
                out.push_str("] => ");
                self.show_into(body, out, false);
            }
            Type::Var(v) => match self.tvars[v].name {
                Some(n) => out.push_str(self.interner.get(n)),
                None => self.show_var(v, out),
            },
            Type::AppVar(v, args) => {
                self.show_var(v, out);
                self.show_args(args, out);
            }
            Type::Union(a, b) => {
                if nested {
                    out.push('(');
                }
                self.show_into(a, out, false);
                out.push_str(" | ");
                self.show_into(b, out, false);
                if nested {
                    out.push(')');
                }
            }
            Type::Inter(a, b) => {
                self.show_into(a, out, true);
                out.push_str(" & ");
                self.show_into(b, out, true);
            }
            Type::Lit(l) => match self.types.lit_val(l) {
                LitVal::Int(i) => out.push_str(&i.to_string()),
                LitVal::Long(l) => {
                    out.push_str(&l.to_string());
                    out.push('L');
                }
                LitVal::Double(bits) => out.push_str(&f64::from_bits(bits).to_string()),
                LitVal::Char(c) => {
                    out.push('\'');
                    out.push(char::from_u32(c as u32).unwrap_or('?'));
                    out.push('\'');
                }
                LitVal::Bool(b) => out.push_str(if b { "true" } else { "false" }),
                LitVal::Str(s) => out.push_str(&format!("{:?}", self.interner.get(s))),
            },
            Type::Blocked(b) => {
                let description = self.types.blocked_description(b);
                out.push_str(description.split_once(": ").map_or(description, |(_, spelling)| spelling));
            }
            Type::This(c) => {
                out.push_str(self.interner.get(self.syms.class(c).name));
                out.push_str(".this");
            }
            Type::Term(_) | Type::Select(..) => {
                out.push('(');
                self.show_path(t, out);
                out.push_str(" : ");
                let under = self.syms.sym(self.path_sym(t)).sig.as_ref().map_or(ERROR, |s| s.ret);
                self.show_into(under, out, false);
                out.push(')');
            }
            Type::Member(p, n) => {
                match self.types.get(p) {
                    Type::Term(_) | Type::Select(..) | Type::This(_) => self.show_path(p, out),
                    Type::Class(c, args) if self.syms.class(c).kind == ClassKind::Object => {
                        out.push_str(self.interner.get(self.syms.class(c).name));
                        self.show_args(args, out);
                    }
                    _ => {
                        self.show_into(p, out, true);
                        out.push('#');
                        out.push_str(self.interner.get(n));
                        return;
                    }
                }
                out.push('.');
                out.push_str(self.interner.get(n));
            }
            // `o.Item`, `Obj.Item`, `O#Item`, as dotty prints a class reference by its prefix.
            Type::Nested(p, c) => {
                match self.types.get(p) {
                    Type::Term(_) | Type::Select(..) | Type::This(_) => {
                        self.show_path(p, out);
                        out.push('.');
                    }
                    Type::Class(k, args) if self.syms.class(k).kind == ClassKind::Object => {
                        out.push_str(self.interner.get(self.syms.class(k).name));
                        self.show_args(args, out);
                        out.push('.');
                    }
                    _ => {
                        self.show_into(p, out, true);
                        out.push('#');
                    }
                }
                self.show_into(c, out, true);
            }
            Type::AppMember(m, args) => {
                self.show_into(m, out, false);
                self.show_args(args, out);
            }
            Type::Decl(a) => out.push_str(self.interner.get(self.syms.aliases[a.idx()].name)),
            Type::Alias(a, args) => {
                out.push_str(self.interner.get(self.syms.aliases[a.idx()].name));
                self.show_args(args, out);
            }
            Type::Match(s, m) => {
                if nested {
                    out.push('(');
                }
                self.show_into(s, out, true);
                out.push_str(" match { ");
                let info = self.types.match_info(m);
                for (i, c) in info.cases.iter().enumerate() {
                    if i > 0 {
                        out.push_str("; ");
                    }
                    out.push_str("case ");
                    if c.pattern == ANY {
                        out.push('_');
                    } else {
                        self.show_into(c.pattern, out, false);
                    }
                    out.push_str(" => ");
                    self.show_into(c.body, out, false);
                }
                out.push_str(" }");
                if nested {
                    out.push(')');
                }
            }
            Type::Refined(..) if self.show_named_function(t, out, nested) => {}
            Type::Refined(..) => {
                let (parent, rs) = self.types.refinements_of(t);
                self.show_into(parent, out, true);
                out.push('{');
                for (i, r) in rs.into_iter().enumerate() {
                    if i > 0 {
                        out.push_str("; ");
                    }
                    self.show_refinement(r, out);
                }
                out.push('}');
            }
        }
    }

    /// `(c: Ctx) => c.T`, a function type refined by an `apply` that names its parameters.
    fn show_named_function(&self, t: TypeId, out: &mut String, nested: bool) -> bool {
        let Type::Refined(parent, r) = self.types.get(t) else { return false };
        let Refinement::Term(crate::names::APPLY, apply, l) = self.types.refinement(r) else { return false };
        let ctx = match self.types.get(parent) {
            Type::Class(c, _) if self.is_function_class(c) => false,
            Type::Class(c, _) if self.is_context_function_class(c) => true,
            _ => return false,
        };
        let Some(sig) = &self.syms.sym(apply).sig else { return false };
        let [clause] = sig.clauses.as_slice() else { return false };
        let types = self.types.items(l);
        if !sig.tparams.is_empty() || types.len() != clause.params.len() + 1 {
            return false;
        }
        if nested {
            out.push('(');
        }
        out.push('(');
        for (i, (p, &ty)) in clause.params.iter().zip(types).enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            out.push_str(self.interner.get(p.name));
            out.push_str(": ");
            self.show_into(ty, out, false);
        }
        out.push_str(if ctx { ") ?=> " } else { ") => " });
        self.show_into(types[clause.params.len()], out, false);
        if nested {
            out.push(')');
        }
        true
    }

    /// `x`, `x.y`, `C.this`.
    fn show_path(&self, t: TypeId, out: &mut String) {
        match self.types.get(t) {
            Type::Term(s) => out.push_str(self.interner.get(self.syms.sym(s).name)),
            Type::Select(p, s) => {
                self.show_path(p, out);
                out.push('.');
                out.push_str(self.interner.get(self.syms.sym(s).name));
            }
            Type::This(c) => {
                out.push_str(self.interner.get(self.syms.class(c).name));
                out.push_str(".this");
            }
            _ => self.show_into(t, out, true),
        }
    }

    fn path_sym(&self, t: TypeId) -> SymId {
        match self.types.get(t) {
            Type::Term(s) | Type::Select(_, s) => s,
            _ => SymId(0),
        }
    }

    fn show_refinement(&self, r: RefineId, out: &mut String) {
        match self.types.refinement(r) {
            Refinement::Alias(n, rhs) => {
                out.push_str("type ");
                out.push_str(self.interner.get(n));
                let rhs = match self.types.get(rhs) {
                    Type::Lambda(ps, body) => {
                        self.show_args(ps, out);
                        body
                    }
                    _ => rhs,
                };
                out.push_str(" = ");
                self.show_into(rhs, out, false);
            }
            Refinement::Bounds(n, lo, hi) => {
                out.push_str("type ");
                out.push_str(self.interner.get(n));
                if lo != NOTHING {
                    out.push_str(" >: ");
                    self.show_into(lo, out, false);
                }
                if hi != ANY {
                    out.push_str(" <: ");
                    self.show_into(hi, out, false);
                }
            }
            Refinement::Val(n, _, ty) => {
                out.push_str("val ");
                out.push_str(self.interner.get(n));
                out.push_str(": ");
                self.show_into(ty, out, false);
            }
            Refinement::Term(n, s, l) => {
                let mut types = self.types.items(l).iter().copied();
                let info = self.syms.sym(s);
                out.push_str(if info.kind == SymKind::Def { "def " } else { "val " });
                out.push_str(self.interner.get(n));
                if let Some(sig) = &info.sig {
                    // The refinement's types begin with its type parameters' bounds.
                    if !sig.tparams.is_empty() {
                        out.push('[');
                        for (i, &t) in sig.tparams.iter().enumerate() {
                            if i > 0 {
                                out.push_str(", ");
                            }
                            out.push_str(self.interner.get(self.syms.tparam(t).name));
                            let (lo, hi) = (types.next().unwrap_or(NOTHING), types.next().unwrap_or(ANY));
                            if lo != NOTHING {
                                out.push_str(" >: ");
                                self.show_into(lo, out, false);
                            }
                            if hi != ANY {
                                out.push_str(" <: ");
                                self.show_into(hi, out, false);
                            }
                        }
                        out.push(']');
                    }
                    for cl in &sig.clauses {
                        out.push('(');
                        if cl.is_using {
                            out.push_str("using ");
                        } else if cl.is_implicit {
                            out.push_str("implicit ");
                        }
                        for (i, p) in cl.params.iter().enumerate() {
                            if i > 0 {
                                out.push_str(", ");
                            }
                            out.push_str(self.interner.get(p.name));
                            out.push_str(if p.by_name { ": => " } else { ": " });
                            self.show_into(types.next().unwrap_or(p.ty), out, false);
                            if p.repeated {
                                out.push('*');
                            }
                        }
                        out.push(')');
                    }
                    out.push_str(": ");
                    self.show_into(types.next().unwrap_or(sig.ret), out, false);
                }
            }
        }
    }

    fn is_function_type(&self, t: TypeId) -> bool {
        match self.types.get(t) {
            Type::Class(c, _) => {
                self.is_function_class(c) || self.is_tuple_class(c)
            }
            _ => false,
        }
    }
}
