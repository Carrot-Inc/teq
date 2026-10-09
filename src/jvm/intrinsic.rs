//! `@jvm("...")` intrinsics. A template is a sequence of JVM instructions separated by blanks:
//!
//! ```text
//! invokevirtual java/lang/String.length()I          one instruction: the arguments are loaded
//!                                                    in order, as the descriptor types them
//! $0 $1:I invokevirtual java/util/ArrayList.get(I)Ljava/lang/Object;
//! getstatic java/lang/System.out:Ljava/io/PrintStream; $0:L invokevirtual java/io/PrintStream.println(Ljava/lang/Object;)V
//! ```
//!
//! `$n` loads argument `n` at its static type, `$n:T` at the type `T` (`I`, `J`, `D`, `Z`, `C`,
//! `L` for `Object`, or a class descriptor). For a member `$0` is the receiver. The instructions
//! are the invokes, field accesses, `new`, `checkcast`, `instanceof`, `ldc "text"`, `ipush n`,
//! the operand-free instructions listed in `plain_op`, and `rt ... rtcall name(desc)`, which
//! calls a definition of `std/jvm.scala` (the dead-code pass reads the name off the template); a template has no branches, so it needs no
//! frames. What the last instruction leaves is the value of the call.
//!
//! The templates the typer makes itself (`$hashCode($0)`, `$pf($0, $1)`, ...) are JS text; the
//! backend knows them by that text.

use super::classfile::*;
use super::gen::{Gen, Invoke};
use super::names::*;
use crate::ast::ListRef;
use crate::symbols::Owner;
use crate::tir::*;
use crate::typer::prims::UNBOX;
use std::rc::Rc;

impl<'a> Gen<'a> {
    /// The reference on the stack as the primitive `prim`, a null as `zero`.
    fn unbox_or(&mut self, prim: &JType, zero: TExprId) {
        let object = JType::object();
        let unbox = self.code.new_label();
        let done = self.code.new_label();
        self.dup();
        self.jump_if(op::IFNONNULL, 1, unbox);
        self.pop_value(&object);
        self.expr(zero, prim);
        self.goto(done);
        self.code.bind(unbox);
        self.adapt(&object, prim);
        self.code.bind(done);
    }

    fn compile_time_only(&mut self) -> JType {
        let exception = "java/lang/UnsupportedOperationException";
        self.new_object(exception);
        self.sconst("runs at compile time only");
        self.invoke(Invoke::Special, exception, false, "<init>", &[JType::L(Rc::from(STRING))], &JType::V);
        self.code.op(op::ATHROW);
        self.code.pop();
        self.code.end_path();
        self.code.push(VT::Null);
        JType::object()
    }

    pub fn intrinsic(&mut self, e: TExprId, template: StrRef, args: ListRef) -> JType {
        let cx = self.cx;
        let text: &str = &cx.input.prog.strings[template.idx()];
        let list = args;
        let args: &[TExprId] = cx.input.prog.expr_list(args);
        if let Some(name) = text.strip_prefix('!') {
            let owner = cx.input.prog.template_syms.get(&template).and_then(|&s| match cx.input.syms.sym(s).owner {
                Owner::Class(c) => Some(self.class_name(c)),
                _ => None,
            });
            // `scala.reflect.Enum.ordinal`, the std's trait standing for the jar's interface, is
            // that interface's method (a macro's `e.ordinal` on an `E <: Enum`).
            if name == "ordinal" && list.len == 1 && owner.as_deref() == Some("scala/reflect/Enum") {
                let iface = JType::L(Rc::from("scala/reflect/Enum"));
                self.expr(args[0], &iface);
                self.invoke(Invoke::Interface, "scala/reflect/Enum", true, "ordinal", &[], &JType::I);
                return JType::I;
            }
            // The std's quoted reflection is the compiler's: a member of it the output keeps
            // (`TreeMethods.asExpr` beside a macro's expansion) never runs.
            if owner.as_deref().map_or(false, |o| o.starts_with("scala/quoted/")) {
                return self.compile_time_only();
            }
            let of = owner.as_deref().map_or(String::new(), |o| format!(" of {}", o));
            self.unsupported(&format!("no @jvm intrinsic or body for {}{}", name, of));
            return JType::V;
        }
        // A quote runs at compile time only: a method holding one that the output keeps (an
        // override of a jar trait's member, `Literally.validate`) throws if it is ever called.
        if text.starts_with("$quote") {
            return self.compile_time_only();
        }
        let object = JType::object();
        match text {
            // The zero has the cast's type, which a later ascription of the cast leaves alone.
            UNBOX => {
                let prim = self.recorded_erasure(args[1]);
                if prim.is_ref() {
                    self.expr(args[0], &prim);
                } else {
                    self.expr(args[0], &object);
                    self.unbox_or(&prim, args[1]);
                }
                return prim;
            }
            "$hashCode($0)" => {
                self.expr(args[0], &object);
                self.invoke(Invoke::Virtual, OBJECT, false, "hashCode", &[], &JType::I);
                return JType::I;
            }
            "$identityHash($0)" => {
                self.expr(args[0], &object);
                self.invoke(Invoke::Static, "java/lang/System", false, "identityHashCode", &[object], &JType::I);
                return JType::I;
            }
            "$getClass($0)" => {
                let class = JType::L(Rc::from("java/lang/Class"));
                // A receiver whose static type is a primitive has the primitive's class, as scalac
                // rewrites it (`1.getClass` prints `int`, `().getClass` `void`); it runs for its
                // effects alone. A value class's is its box's, which the receiver is boxed into.
                if let Some(t) = self.recorded_type(args[0]).filter(|_| self.expr_value_class(args[0]).is_none()) {
                    let erased = self.erase(t);
                    if !matches!(erased, JType::L(_)) {
                        self.statement(args[0]);
                        let boxed = if erased == JType::V { "java/lang/Void" } else { erased.box_class() };
                        self.getstatic(boxed, "TYPE", &class);
                        return class;
                    }
                }
                self.expr(args[0], &object);
                self.invoke(Invoke::Virtual, OBJECT, false, "getClass", &[], &class);
                return class;
            }
            "$equals($0, $1)" => {
                self.expr(args[0], &object);
                self.expr(args[1], &object);
                self.invoke(Invoke::Virtual, OBJECT, false, "equals", &[object], &JType::Z);
                return JType::Z;
            }
            "$doubleEquals($0, $1)" => {
                let double = JType::L(Rc::from("java/lang/Double"));
                self.expr(args[0], &JType::D);
                self.adapt(&JType::D, &double);
                self.expr(args[1], &object);
                self.invoke(Invoke::Virtual, OBJECT, false, "equals", &[object], &JType::Z);
                return JType::Z;
            }
            "$enumValueOf($0, $1, $2)" | "$enumFromOrdinal($0, $1, $2)" => {
                let by_name = text.starts_with("$enumValueOf");
                let array = JType::L(Rc::from(OBJECT_ARRAY));
                let string = JType::L(Rc::from(STRING));
                self.expr(args[0], &array);
                let key = if by_name { string.clone() } else { JType::I };
                self.expr(args[1], &key);
                self.expr(args[2], &string);
                return self.helper_call(if by_name { "enumValueOf" } else { "enumFromOrdinal" }, 3);
            }
            "$productPrefix($0)" => {
                let product = JType::L(Rc::from(PRODUCT));
                self.expr(args[0], &product);
                let string = JType::L(Rc::from(STRING));
                self.invoke(Invoke::Interface, PRODUCT, true, "productPrefix", &[], &string);
                return string;
            }
            "$0.$ordinal" => {
                let t = JType::L(Rc::from(ENUM));
                self.expr(args[0], &t);
                self.invoke(Invoke::Interface, ENUM, true, "ordinal", &[], &JType::I);
                return JType::I;
            }
            "$pf($0, $1)" => {
                let Some(&c) = cx.helpers.classes.get("PartialFunctionImpl") else {
                    self.unsupported("the runtime class PartialFunctionImpl is missing");
                    return JType::V;
                };
                let name = self.class_name(c);
                self.new_object(&name);
                let params = self.ctor_param_types(c);
                for (a, p) in args.iter().zip(&params) {
                    self.expr(*a, p);
                }
                self.invoke(Invoke::Special, &name, false, "<init>", &params, &JType::V);
                return JType::L(name);
            }
            _ => {}
        }
        if text.starts_with('$') && !text[1..].starts_with(|c: char| c.is_ascii_digit()) || text.starts_with('(') {
            self.unsupported(&format!("the JavaScript template {}", text));
            return JType::V;
        }
        self.assemble(e, text, args)
    }

    fn assemble(&mut self, e: TExprId, text: &str, args: &[TExprId]) -> JType {
        let tokens = tokenize(text);
        let mut stack: Vec<JType> = Vec::new();
        let single = !text.contains('$') && tokens.len() == 2 && tokens[0].starts_with("invoke");
        let mut i = 0;
        while i < tokens.len() {
            let t = tokens[i].as_str();
            i += 1;
            if let Some(rest) = t.strip_prefix('$') {
                let (index, ty) = match rest.split_once(':') {
                    Some((n, ty)) => (n, Some(ty)),
                    None => (rest, None),
                };
                let Some(&arg) = index.parse::<usize>().ok().and_then(|n| args.get(n)) else {
                    self.unsupported(&format!("the placeholder {} of {}", t, text));
                    return JType::V;
                };
                let want = match ty {
                    Some("L") => JType::object(),
                    Some(d) => parse_type(d).0,
                    // A value class's instance is its box, the parameter being generic (the
                    // element `update` stores in a `V[]`): no std intrinsic declares one.
                    None => match self.expr_value_class(arg) {
                        Some((c, _)) => self.class_type(c),
                        None => {
                            let s = self.static_type(arg);
                            self.materialised_type(s)
                        }
                    },
                };
                self.expr(arg, &want);
                stack.push(want);
                continue;
            }
            let mut operand = || {
                let o = tokens.get(i).cloned().unwrap_or_default();
                i += 1;
                o
            };
            match t {
                "invokestatic" | "invokevirtual" | "invokeinterface" | "invokespecial" => {
                    let member = operand();
                    let Some((owner, name, desc)) = split_method(&member) else {
                        self.unsupported(&format!("the method reference {} of {}", member, text));
                        return JType::V;
                    };
                    let (params, ret) = parse_method_desc(desc);
                    let kind = match t {
                        "invokestatic" => Invoke::Static,
                        "invokevirtual" => Invoke::Virtual,
                        "invokeinterface" => Invoke::Interface,
                        _ => Invoke::Special,
                    };
                    if single {
                        let mut wanted: Vec<JType> = Vec::new();
                        if kind != Invoke::Static {
                            wanted.push(JType::L(Rc::from(owner)));
                        }
                        wanted.extend(params.iter().cloned());
                        let skip = args.len().saturating_sub(wanted.len());
                        for (a, w) in args[skip..].iter().zip(&wanted) {
                            self.expr(*a, w);
                        }
                    } else {
                        let n = params.len() + (kind != Invoke::Static) as usize;
                        stack.truncate(stack.len().saturating_sub(n));
                    }
                    self.invoke_desc(kind, owner, kind == Invoke::Interface, name, desc, params.len(), &ret);
                    if ret != JType::V {
                        stack.push(ret);
                    }
                }
                "getstatic" | "getfield" | "putstatic" | "putfield" => {
                    let member = operand();
                    let Some((owner_name, desc)) = member.split_once(':') else {
                        self.unsupported(&format!("the field reference {} of {}", member, text));
                        return JType::V;
                    };
                    let Some((owner, name)) = owner_name.rsplit_once('.') else {
                        self.unsupported(&format!("the field reference {} of {}", member, text));
                        return JType::V;
                    };
                    let ty = parse_type(desc).0;
                    match t {
                        "getstatic" => {
                            self.getstatic(owner, name, &ty);
                            stack.push(ty);
                        }
                        "getfield" => {
                            self.getfield(owner, name, &ty);
                            stack.pop();
                            stack.push(ty);
                        }
                        "putstatic" => {
                            self.putstatic(owner, name, &ty);
                            stack.pop();
                        }
                        _ => {
                            self.putfield(owner, name, &ty);
                            stack.pop();
                            stack.pop();
                        }
                    }
                }
                "new" => {
                    let class = operand();
                    let offset = self.code.pc() as u16;
                    let index = self.cw.cp.class(&class);
                    self.code.op_u16(op::NEW, index);
                    self.code.push(VT::Uninit(offset));
                    stack.push(JType::L(Rc::from(class.as_str())));
                }
                "checkcast" => {
                    let class = operand();
                    self.checkcast(&class);
                    stack.pop();
                    stack.push(JType::L(Rc::from(class.as_str())));
                }
                "instanceof" => {
                    let class = operand();
                    self.instance_of(&class);
                    stack.pop();
                    stack.push(JType::Z);
                }
                "rt" => {
                    // The module of std/jvm.scala, the receiver of a following `rtcall`.
                    let module = JType::L(Rc::from(RUNTIME_MODULE));
                    self.getstatic(RUNTIME_MODULE, "MODULE$", &module);
                    stack.push(module);
                }
                "rtcall" => {
                    let member = operand();
                    let Some(paren) = member.find('(') else {
                        self.unsupported(&format!("the runtime call {} of {}", member, text));
                        return JType::V;
                    };
                    let (name, desc) = member.split_at(paren);
                    let (params, ret) = parse_method_desc(desc);
                    stack.truncate(stack.len().saturating_sub(params.len() + 1));
                    self.invoke_desc(Invoke::Virtual, RUNTIME_MODULE, false, name, desc, params.len(), &ret);
                    if ret != JType::V {
                        stack.push(ret);
                    }
                }
                "ipush" => {
                    let n: i32 = operand().parse().unwrap_or(0);
                    self.iconst(n);
                    stack.push(JType::I);
                }
                "ldc" => {
                    let s = operand();
                    self.sconst(&s);
                    stack.push(JType::L(Rc::from(STRING)));
                }
                "dup" => {
                    self.dup();
                    if let Some(top) = stack.last().cloned() {
                        stack.push(top);
                    }
                }
                "pop" => {
                    if let Some(top) = stack.pop() {
                        self.pop_value(&top);
                    }
                }
                // Arrays in link mode, by the type of the array operand: the instruction of a
                // JVM array's kind, `ScalaRunTime`'s method of the name on an erased one.
                "array_apply" => {
                    let _ = stack.pop();
                    let array = stack.pop().unwrap_or_else(JType::object);
                    let elem = self.array_load(&array);
                    stack.push(elem);
                }
                "array_update" => {
                    let value = stack.pop().unwrap_or_else(JType::object);
                    let _ = stack.pop();
                    let array = stack.pop().unwrap_or_else(JType::object);
                    self.array_store(&array, &value);
                }
                "array_length" => {
                    let array = stack.pop().unwrap_or_else(JType::object);
                    self.array_length(&array);
                    stack.push(JType::I);
                }
                "array_clone" => {
                    let array = stack.pop().unwrap_or_else(JType::object);
                    let t = self.array_clone(&array);
                    stack.push(t);
                }
                // `java.lang.StringBuilder`'s `append` and `insert` of the operand's own type: the
                // builder (and the index) below the value.
                "builder_append" | "builder_insert" => {
                    let value = stack.pop().unwrap_or_else(JType::object);
                    let param = Self::rendered_as(&value);
                    let builder = JType::L(Rc::from(STRING_BUILDER));
                    let params = if t == "builder_insert" { vec![JType::I, param] } else { vec![param] };
                    stack.truncate(stack.len().saturating_sub(params.len()));
                    self.invoke(Invoke::Virtual, STRING_BUILDER, false, &t["builder_".len()..], &params, &builder);
                    stack.push(builder);
                }
                // The value on the stack cast to the type the call has (an array a runtime
                // helper made as an `Object`).
                "cast_result" => {
                    let from = stack.pop().unwrap_or_else(JType::object);
                    let to = self.recorded_erasure(e);
                    self.adapt(&from, &to);
                    stack.push(to);
                }
                // A new array of the type the call has, its length on the stack.
                "array_new" => {
                    let _ = stack.pop();
                    let array = self.recorded_erasure(e);
                    self.new_array(&array);
                    stack.push(array);
                }
                _ => match plain_op(t) {
                    Some((opcode, pops, push)) => {
                        self.code.op(opcode);
                        self.code.popn(pops);
                        if opcode == op::ATHROW {
                            self.code.end_path();
                        }
                        stack.truncate(stack.len().saturating_sub(pops));
                        if let Some(p) = push {
                            let vt = self.vt(&p);
                            self.code.push(vt);
                            stack.push(p);
                        }
                    }
                    None => {
                        self.unsupported(&format!("the instruction {} of {}", t, text));
                        return JType::V;
                    }
                },
            }
        }
        stack.pop().unwrap_or(JType::V)
    }

    fn materialised_type(&mut self, t: JType) -> JType {
        if t == JType::V {
            JType::L(Rc::from(BOXED_UNIT))
        } else {
            t
        }
    }
}

/// Blank-separated tokens; a double-quoted token may hold blanks and the escapes `\n`, `\"`, `\\`.
fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else if c == '"' {
            chars.next();
            let mut s = String::new();
            while let Some(c) = chars.next() {
                match c {
                    '"' => break,
                    '\\' => match chars.next() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some(other) => s.push(other),
                        None => {}
                    },
                    _ => s.push(c),
                }
            }
            out.push(s);
        } else {
            let mut s = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    break;
                }
                s.push(c);
                chars.next();
            }
            out.push(s);
        }
    }
    out
}

/// `owner.name(desc)ret` into its parts.
fn split_method(member: &str) -> Option<(&str, &str, &str)> {
    let paren = member.find('(')?;
    let dot = member[..paren].rfind('.')?;
    Some((&member[..dot], &member[dot + 1..paren], &member[paren..]))
}

/// The instructions without operands: opcode, how many values they pop, what they push.
fn plain_op(name: &str) -> Option<(u8, usize, Option<JType>)> {
    use JType::*;
    Some(match name {
        "aconst_null" => (op::ACONST_NULL, 0, Some(JType::object())),
        "iconst_0" => (op::ICONST_0, 0, Some(I)),
        "iconst_1" => (op::ICONST_0 + 1, 0, Some(I)),
        "iconst_m1" => (op::ICONST_0 - 1, 0, Some(I)),
        "iadd" => (op::IADD, 2, Some(I)),
        "isub" => (op::ISUB, 2, Some(I)),
        "imul" => (op::IMUL, 2, Some(I)),
        "iand" => (op::IAND, 2, Some(I)),
        "ior" => (op::IOR, 2, Some(I)),
        "ixor" => (op::IXOR, 2, Some(I)),
        "ishl" => (op::ISHL, 2, Some(I)),
        "ishr" => (op::ISHR, 2, Some(I)),
        "iushr" => (op::IUSHR, 2, Some(I)),
        "ineg" => (op::INEG, 1, Some(I)),
        "ladd" => (op::LADD, 2, Some(J)),
        "lsub" => (op::LSUB, 2, Some(J)),
        "lmul" => (op::LMUL, 2, Some(J)),
        "land" => (op::LAND, 2, Some(J)),
        "lor" => (op::LOR, 2, Some(J)),
        "lxor" => (op::LXOR, 2, Some(J)),
        "lneg" => (op::LNEG, 1, Some(J)),
        "dadd" => (op::DADD, 2, Some(D)),
        "dsub" => (op::DSUB, 2, Some(D)),
        "dmul" => (op::DMUL, 2, Some(D)),
        "ddiv" => (op::DDIV, 2, Some(D)),
        "dneg" => (op::DNEG, 1, Some(D)),
        "i2l" => (op::I2L, 1, Some(J)),
        "i2d" => (op::I2D, 1, Some(D)),
        "i2c" => (op::I2C, 1, Some(C)),
        "l2i" => (op::L2I, 1, Some(I)),
        "l2d" => (op::L2D, 1, Some(D)),
        "d2i" => (op::D2I, 1, Some(I)),
        "d2f" => (op::D2F, 1, Some(F)),
        "f2d" => (op::F2D, 1, Some(D)),
        "i2f" => (op::I2F, 1, Some(F)),
        "f2i" => (op::F2I, 1, Some(I)),
        "i2b" => (op::I2B, 1, Some(B)),
        "i2s" => (op::I2S, 1, Some(S)),
        "d2l" => (op::D2L, 1, Some(J)),
        "lcmp" => (op::LCMP, 2, Some(I)),
        "dcmpl" => (op::DCMPL, 2, Some(I)),
        "dcmpg" => (op::DCMPG, 2, Some(I)),
        "athrow" => (op::ATHROW, 1, None),
        _ => return None,
    })
}
