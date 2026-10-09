//! The machine over generated programs: every rule is an edit a person makes between two
//! builds, decided from the model as it stands, and after every rule the session's build is
//! held against a fresh one (`oracle`).

use crate::driver::{Due, Plan};
use crate::model::{Def, Expr, Fault, Kind, Macro, Owner, Param, Program, Shape, Stmt, Ty};
use crate::session;
use hegel::generators as gs;
use hegel::stateful::{Invariant, Rule, StateMachine};
use hegel::TestCase;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

pub struct Edits {
    pub program: Program,
    pub driver: Rc<RefCell<Plan>>,
    rules: Vec<(&'static str, f64)>,
    /// Whether an error in a signature or an import goes in alone: repaired in the next build
    /// with nothing else changed, or as the history's last step (`known`,
    /// `signature-error-lost`).
    bodies_alone: bool,
    /// Whether the step being written down is the history's last.
    ending: bool,
}

/// The rules and their weights. The edits that stay on the incremental path weigh most: a
/// campaign whose builds are nearly all full has not attacked what a retype keeps.
pub const RULES: [(&str, f64); 10] = [
    ("literal", 5.0),
    ("statement", 3.0),
    ("two_files", 2.0),
    ("nothing", 1.0),
    ("revert", 1.0),
    ("signature", 1.0),
    ("definition", 1.0),
    ("file", 0.5),
    ("fault", 0.7),
    ("repair", 3.0),
];

/// The three rules of the pilot.
pub const PILOT: [(&str, f64); 3] = [("literal", 1.0), ("statement", 1.0), ("nothing", 1.0)];

const WORDS: [&str; 8] = ["a", "bb", "ccc", "one", "two", "three", "dddd", "ee"];
const TEXTS: [&str; 10] = ["", "a", "two words", "é", "𝄞", "q\"uote", "back\\slash", "line\nbreak", "$dollar", "tab\there"];
const NUMBERS: [i32; 10] = [0, 1, 2, 3, 7, 10, -1, 255, i32::MAX, i32::MIN];

fn index(tc: &TestCase, count: usize) -> usize {
    tc.draw(gs::integers::<usize>().min_value(0).max_value(count - 1))
}

fn number(tc: &TestCase) -> i32 {
    NUMBERS[index(tc, NUMBERS.len())]
}

fn text(tc: &TestCase) -> String {
    TEXTS[index(tc, TEXTS.len())].to_string()
}

fn words(tc: &TestCase) -> String {
    let count = 1 + index(tc, 3);
    let chosen: Vec<&str> = (0..count).map(|_| WORDS[index(tc, WORDS.len())]).collect();
    chosen.join(" ")
}

fn ty(tc: &TestCase) -> Ty {
    [Ty::Int, Ty::Str][index(tc, 2)]
}

/// What an expression may name where it stands.
struct Place {
    caller: usize,
    params: usize,
    locals: Vec<usize>,
    in_class: bool,
    /// An inline method of an object or a class, whose body calls nothing and runs no macro
    /// (`known`, `macro-under-receiver`).
    plain: bool,
}

fn plain(owner: Owner, kind: Kind) -> bool {
    kind == Kind::Inline && owner != Owner::Top && !crate::known::asked("macro-under-receiver")
}

fn place(def: &Def) -> Place {
    let locals = def.shape.statements.iter().filter_map(|statement| match statement {
        Stmt::Val(n, _) => Some(*n),
        Stmt::Print(_) => None,
    });
    Place {
        caller: def.id,
        params: if def.kind == Kind::Main { 0 } else { def.shape.params.len() },
        locals: locals.collect(),
        in_class: def.owner == Owner::Class,
        plain: plain(def.owner, def.kind),
    }
}

fn expr(tc: &TestCase, program: &Program, place: &Place, depth: usize) -> Expr {
    let callees: Vec<&Def> = program
        .defs
        .iter()
        .filter(|def| def.kind != Kind::Main && (place.caller == 0 || def.id < place.caller))
        .filter(|_| !place.plain)
        .collect();
    let mut forms = vec!["int", "text"];
    if !place.plain {
        forms.push("macro");
    }
    if place.params > 0 {
        forms.push("param");
    }
    if !callees.is_empty() {
        forms.push("call");
    }
    if depth > 0 {
        forms.push("add");
    }
    if !place.locals.is_empty() {
        forms.push("local");
    }
    if place.in_class {
        forms.push("seed");
    }
    match forms[index(tc, forms.len())] {
        "int" => Expr::Int(number(tc)),
        "text" => Expr::Str(text(tc)),
        "macro" => Expr::Macro([Macro::Count, Macro::Shape, Macro::Words][index(tc, 3)], words(tc)),
        "param" => Expr::Param(index(tc, place.params)),
        "local" => Expr::Local(place.locals[index(tc, place.locals.len())]),
        "seed" => Expr::Seed,
        "add" => {
            let of = ty(tc);
            Expr::Add(of, Box::new(expr(tc, program, place, depth - 1)), Box::new(expr(tc, program, place, depth - 1)))
        }
        _ => {
            let callee = callees[index(tc, callees.len())];
            let given = index(tc, callee.shape.params.len() + 1);
            let args = (0..given).map(|_| expr(tc, program, place, depth.saturating_sub(1))).collect();
            Expr::Call { def: callee.id, seed: number(tc), args }
        }
    }
}

fn shape(tc: &TestCase, program: &Program, caller: usize, owner: Owner, kind: Kind) -> Shape {
    let count = if kind == Kind::Val { 0 } else { index(tc, 3) };
    let params = (0..count)
        .map(|_| {
            let of = ty(tc);
            let default = tc.draw(gs::booleans()).then(|| match of {
                Ty::Int => Expr::Int(number(tc)),
                Ty::Str => Expr::Str(text(tc)),
            });
            Param { ty: of, default }
        })
        .collect();
    let place = Place { caller, params: count, locals: Vec::new(), in_class: owner == Owner::Class, plain: plain(owner, kind) };
    Shape {
        params,
        result: ty(tc),
        declared: kind == Kind::Inline || tc.draw(gs::booleans()),
        statements: Vec::new(),
        value: expr(tc, program, &place, 2),
    }
}

fn new_def(tc: &TestCase, program: &mut Program, file: usize) -> usize {
    let kind = [Kind::Def, Kind::Inline, Kind::Val][index(tc, 3)];
    let owner = [Owner::Top, Owner::Object, Owner::Class][index(tc, 3)];
    let made = shape(tc, program, program.next_def, owner, kind);
    program.add_def(file, owner, kind, made)
}

impl Edits {
    /// A drawn program of two to four files with one to three definitions each.
    pub fn drawn(tc: &TestCase, kind: session::Kind, test: &str, rules: &[(&'static str, f64)]) -> Edits {
        let mut program = Program::new();
        for _ in 0..2 + index(tc, 3) {
            let file = program.add_file(index(tc, 3));
            for _ in 0..1 + index(tc, 3) {
                new_def(tc, &mut program, file);
            }
        }
        Edits::over(program, kind, test, rules)
    }

    /// The pilot's program: two packages, a macro's call in three files, an inline method
    /// called from another file, a class and an object.
    pub fn small(kind: session::Kind, test: &str, rules: &[(&'static str, f64)]) -> Edits {
        let mut program = Program::new();
        let int = |n: i32| Expr::Int(n);
        let call = |def: usize, args: Vec<Expr>| Expr::Call { def, seed: 3, args };
        let add = |of: Ty, a: Expr, b: Expr| Expr::Add(of, Box::new(a), Box::new(b));
        let one = |of: Ty| vec![Param { ty: of, default: None }];
        let first = program.add_file(0);
        let second = program.add_file(1);
        let shapes = [
            (first, Owner::Top, Kind::Def, one(Ty::Int), Ty::Int, add(Ty::Int, Expr::Param(0), int(2))),
            (first, Owner::Object, Kind::Def, one(Ty::Str), Ty::Str, add(Ty::Str, Expr::Param(0), Expr::Macro(Macro::Shape, "a bb".to_string()))),
            (first, Owner::Top, Kind::Inline, one(Ty::Int), Ty::Int, add(Ty::Int, Expr::Param(0), Expr::Macro(Macro::Count, "one two".to_string()))),
            (second, Owner::Top, Kind::Def, one(Ty::Int), Ty::Int, add(Ty::Int, call(1, vec![Expr::Param(0)]), call(3, vec![int(1)]))),
            (second, Owner::Class, Kind::Def, one(Ty::Int), Ty::Str, add(Ty::Str, Expr::Seed, call(2, vec![Expr::Str("s".to_string())]))),
            (second, Owner::Object, Kind::Val, Vec::new(), Ty::Int, add(Ty::Int, Expr::Macro(Macro::Words, "four five".to_string()), call(4, vec![int(7)]))),
        ];
        for (file, owner, kind, params, result, value) in shapes {
            let shape = Shape { params, result, declared: true, statements: Vec::new(), value };
            program.add_def(file, owner, kind, shape);
        }
        Edits::over(program, kind, test, rules)
    }

    fn over(program: Program, kind: session::Kind, test: &str, rules: &[(&'static str, f64)]) -> Edits {
        let driver = Rc::new(RefCell::new(Plan::new(kind, test, program.render())));
        let bodies_alone = kind == session::Kind::Check && !crate::known::asked("signature-error-lost");
        Edits { program, driver, rules: rules.to_vec(), bodies_alone, ending: false }
    }

    fn touch(&mut self, id: usize) -> &mut Shape {
        let def = self.program.def_mut(id);
        def.previous = Some(def.shape.clone());
        &mut def.shape
    }

    fn some_def(&self, tc: &TestCase, with_entry: bool) -> Option<usize> {
        let ids: Vec<usize> = self.program.defs.iter().filter(|def| with_entry || def.kind != Kind::Main).map(|def| def.id).collect();
        (!ids.is_empty()).then(|| ids[index(tc, ids.len())])
    }

    /// Changes a literal of the definition, the one the index falls on; false when it has none.
    fn change_literal(&mut self, tc: &TestCase, id: usize) -> bool {
        let count = self.program.def_mut(id).shape.literals().len();
        if count == 0 {
            return false;
        }
        let site = index(tc, count);
        let old = self.program.def_mut(id).shape.literals()[site].clone();
        let new = match old {
            Expr::Int(_) => Expr::Int(number(tc)),
            Expr::Str(_) => Expr::Str(text(tc)),
            Expr::Macro(which, _) => Expr::Macro(which, words(tc)),
            _ => unreachable!(),
        };
        let name = self.name_of(id);
        self.driver.borrow_mut().say(format!("literal {site} of {name} becomes {new:?}"));
        *self.touch(id).literals()[site] = new;
        true
    }

    fn macro_files(&self) -> BTreeSet<String> {
        self.program.files.iter().filter(|file| self.program.calls_macro(file.id)).map(|file| file.name()).collect()
    }

    fn name_of(&self, id: usize) -> String {
        let def = self.program.def(id).unwrap();
        let file = self.program.file(def.file).name();
        match def.kind {
            Kind::Main => format!("main in {file}"),
            _ => format!("d{id} in {file}"),
        }
    }
}

fn literal(m: &mut Edits, tc: TestCase) {
    let id = m.some_def(&tc, true).unwrap();
    if !m.change_literal(&tc, id) {
        m.driver.borrow_mut().say("a definition without literals: nothing changes".to_string());
    }
    m.driver.borrow_mut().rule("literal", Due::Changed);
}

fn statement(m: &mut Edits, tc: TestCase) {
    let id = m.some_def(&tc, true).unwrap();
    let count = m.program.def(id).unwrap().shape.statements.len();
    let name = m.name_of(id);
    if count > 0 && index(&tc, 3) == 0 {
        let at = index(&tc, count);
        m.driver.borrow_mut().say(format!("statement {at} of {name} is taken out"));
        m.touch(id).statements.remove(at);
    } else {
        let at = index(&tc, count + 1);
        let value = expr(&tc, &m.program, &place(m.program.def(id).unwrap()), 1);
        if tc.draw(gs::booleans()) {
            m.driver.borrow_mut().say(format!("println({value:?}) goes into {name} at {at}"));
            m.touch(id).statements.insert(at, Stmt::Print(value));
        } else {
            let local = m.program.fresh_local();
            let used = tc.draw(gs::booleans());
            m.driver.borrow_mut().say(format!("val t{local} = {value:?} goes into {name} at {at}{}", if used { ", and into its value" } else { "" }));
            let shape = m.touch(id);
            shape.statements.insert(at, Stmt::Val(local, value));
            if used {
                let old = std::mem::replace(&mut shape.value, Expr::Int(0));
                shape.value = Expr::Add(shape.result, Box::new(Expr::Local(local)), Box::new(old));
            }
        }
    }
    m.driver.borrow_mut().rule("statement", Due::Changed);
}

fn two_files(m: &mut Edits, tc: TestCase) {
    let first = m.some_def(&tc, true).unwrap();
    let file = m.program.def(first).unwrap().file;
    let others: Vec<usize> = m.program.defs.iter().filter(|def| def.file != file).map(|def| def.id).collect();
    m.change_literal(&tc, first);
    if !others.is_empty() {
        let second = others[index(&tc, others.len())];
        m.change_literal(&tc, second);
    }
    m.driver.borrow_mut().rule("two_files", Due::Changed);
}

fn nothing(m: &mut Edits, tc: TestCase) {
    let names: Vec<String> = m.program.render().into_keys().collect();
    let due = match index(&tc, names.len() + 1) {
        0 => Due::Plain,
        n => Due::Unchanged(names[n - 1].clone()),
    };
    m.driver.borrow_mut().say(format!("nothing changes, {due:?}"));
    m.driver.borrow_mut().rule("nothing", due);
}

fn revert(m: &mut Edits, tc: TestCase) {
    let ids: Vec<usize> = m.program.defs.iter().filter(|def| def.previous.is_some()).map(|def| def.id).collect();
    if ids.is_empty() {
        m.driver.borrow_mut().say("nothing to revert: nothing changes".to_string());
    } else {
        let id = ids[index(&tc, ids.len())];
        m.driver.borrow_mut().say(format!("{} goes back to what it was before its last edit", m.name_of(id)));
        let def = m.program.def_mut(id);
        let previous = def.previous.take().unwrap();
        def.previous = Some(std::mem::replace(&mut def.shape, previous));
    }
    m.driver.borrow_mut().rule("revert", Due::Changed);
}

fn signature(m: &mut Edits, tc: TestCase) {
    let Some(id) = m.some_def(&tc, false) else {
        m.driver.borrow_mut().say("no definition to change the signature of".to_string());
        return m.driver.borrow_mut().rule("signature", Due::Changed);
    };
    let def = m.program.def(id).unwrap();
    let (kind, params) = (def.kind, def.shape.params.len());
    let natural = m.program.result_of(def);
    let mut changes = vec!["result"];
    if kind != Kind::Inline {
        changes.push("declared");
    }
    if kind != Kind::Val {
        changes.push("parameter added");
        if params > 0 {
            changes.push("parameter dropped");
        }
    }
    let change = changes[index(&tc, changes.len())];
    let added = Param { ty: ty(&tc), default: tc.draw(gs::booleans()).then(|| Expr::Int(number(&tc))) };
    m.driver.borrow_mut().say(format!("signature of {}: {change}", m.name_of(id)));
    let shape = m.touch(id);
    match change {
        "result" => {
            shape.declared = true;
            shape.result = if natural == Ty::Int { Ty::Str } else { Ty::Int };
        }
        "declared" => {
            shape.declared = !shape.declared;
            shape.result = natural;
        }
        "parameter added" => shape.params.push(added),
        _ => {
            shape.params.pop();
        }
    }
    m.driver.borrow_mut().rule("signature", Due::Changed);
}

fn definition(m: &mut Edits, tc: TestCase) {
    let removable = m.program.defs.len() > 2;
    if removable && index(&tc, 3) == 0 {
        let id = m.some_def(&tc, false).unwrap();
        m.driver.borrow_mut().say(format!("{} is removed, its calls become literals", m.name_of(id)));
        m.program.remove_def(id);
    } else {
        let files: Vec<usize> = m.program.files.iter().filter(|file| file.id != 0).map(|file| file.id).collect();
        let file = match files.is_empty() {
            true => m.program.add_file(index(&tc, 3)),
            false => files[index(&tc, files.len())],
        };
        let id = new_def(&tc, &mut m.program, file);
        m.driver.borrow_mut().say(format!("{} is added: {:?}", m.name_of(id), m.program.def(id).unwrap().shape));
    }
    m.driver.borrow_mut().rule("definition", Due::Changed);
}

fn file(m: &mut Edits, tc: TestCase) {
    let files: Vec<usize> = m.program.files.iter().filter(|file| file.id != 0).map(|file| file.id).collect();
    let before = m.program.render();
    let first = if files.len() > 1 && index(&tc, 2) == 0 {
        let id = files[index(&tc, files.len())];
        let name = m.program.file(id).name();
        let emptied = m.program.file(id).emptied();
        m.driver.borrow_mut().say(format!("{name} is removed with its definitions"));
        m.program.remove_file(id);
        // The definitions go first, with their callers, the file after them: the program of
        // the first build is the final one plus a file of a package clause and an import.
        let mut first = m.program.render();
        first.insert(name.clone(), emptied);
        first
    } else {
        let id = m.program.add_file(index(&tc, 3));
        let def = new_def(&tc, &mut m.program, id);
        m.driver.borrow_mut().say(format!("{} is added: {:?}", m.name_of(def), m.program.def(def).unwrap().shape));
        // The file first, its callers after it.
        let name = m.program.file(id).name();
        let mut first = before;
        first.insert(name.clone(), m.program.render()[&name].clone());
        first
    };
    if !crate::known::asked("file-change-with-edit") {
        let mut plan = m.driver.borrow_mut();
        plan.rule("file", Due::Changed);
        plan.step(first, m.macro_files(), m.program.faulty());
    }
    m.driver.borrow_mut().rule("file", Due::Changed);
}

fn fault(m: &mut Edits, tc: TestCase) {
    let sound: Vec<usize> = m.program.defs.iter().filter(|def| def.fault.is_none() && def.kind != Kind::Main).map(|def| def.id).collect();
    let files: Vec<usize> = m.program.files.iter().filter(|file| !file.bad_import).map(|file| file.id).collect();
    let choice = index(&tc, 4);
    if choice == 3 && !files.is_empty() {
        let id = files[index(&tc, files.len())];
        m.driver.borrow_mut().say(format!("{} imports what does not exist", m.program.file(id).name()));
        m.program.files.iter_mut().find(|file| file.id == id).unwrap().bad_import = true;
        if m.bodies_alone {
            return alone(m, tc, |m| m.program.files.iter_mut().find(|file| file.id == id).unwrap().bad_import = false);
        }
    } else if !sound.is_empty() {
        let id = sound[index(&tc, sound.len())];
        let def = m.program.def(id).unwrap();
        let typed = def.shape.declared || def.kind == Kind::Inline;
        let fault = match choice {
            0 if typed => Fault::Mismatch,
            2 => Fault::Signature,
            _ => Fault::Unknown,
        };
        m.driver.borrow_mut().say(format!("{} gets an error: {fault:?}", m.name_of(id)));
        m.program.def_mut(id).fault = Some(fault);
        if m.bodies_alone && fault == Fault::Signature {
            return alone(m, tc, |m| m.program.def_mut(id).fault = None);
        }
    } else {
        m.driver.borrow_mut().say("nothing left to break: nothing changes".to_string());
    }
    m.driver.borrow_mut().rule("fault", Due::Changed);
}

/// A check session's error in a signature or an import, just injected: its build is written
/// down, and the error is repaired in the next build with nothing else changed, or it stands
/// as the history's last step.
fn alone(m: &mut Edits, tc: TestCase, repair: impl FnOnce(&mut Edits)) {
    m.driver.borrow_mut().rule("fault", Due::Changed);
    if tc.draw(gs::weighted_booleans(0.2)) {
        m.driver.borrow_mut().say("the history ends with that error standing".to_string());
        m.ending = true;
        return;
    }
    m.driver.borrow_mut().step(m.program.render(), m.macro_files(), m.program.faulty());
    m.driver.borrow_mut().say("that error is repaired, nothing else changes".to_string());
    repair(m);
    m.driver.borrow_mut().rule("repair", Due::Changed);
}

fn repair(m: &mut Edits, tc: TestCase) {
    let broken: Vec<usize> = m.program.defs.iter().filter(|def| def.fault.is_some()).map(|def| def.id).collect();
    let files: Vec<usize> = m.program.files.iter().filter(|file| file.bad_import).map(|file| file.id).collect();
    if broken.is_empty() && files.is_empty() {
        m.driver.borrow_mut().say("nothing to repair: nothing changes".to_string());
    } else if tc.draw(gs::booleans()) {
        m.driver.borrow_mut().say("every error is repaired".to_string());
        m.program.defs.iter_mut().for_each(|def| def.fault = None);
        m.program.files.iter_mut().for_each(|file| file.bad_import = false);
    } else {
        let at = index(&tc, broken.len() + files.len());
        if at < broken.len() {
            m.driver.borrow_mut().say(format!("the error of {} is repaired", m.name_of(broken[at])));
            m.program.def_mut(broken[at]).fault = None;
        } else {
            let id = files[at - broken.len()];
            m.driver.borrow_mut().say(format!("the import of {} is taken out", m.program.file(id).name()));
            m.program.files.iter_mut().find(|file| file.id == id).unwrap().bad_import = false;
        }
    }
    m.driver.borrow_mut().rule("repair", Due::Changed);
}

fn written(m: &mut Edits, _: TestCase) {
    let mut plan = m.driver.borrow_mut();
    plan.step(m.program.render(), m.macro_files(), m.program.faulty());
    if m.ending {
        plan.end();
    }
}

impl Edits {
    /// Draws a history with the machine and runs it.
    pub fn run(self, tc: TestCase, steps: i64) {
        let plan = self.driver.clone();
        hegel::stateful::machine(self).steps(steps).run(tc);
        plan.borrow().run();
    }
}

impl StateMachine for Edits {
    fn rules(&self) -> Vec<Rule<Self>> {
        let apply = |name: &str| -> fn(&mut Edits, TestCase) {
            match name {
                "literal" => literal,
                "statement" => statement,
                "two_files" => two_files,
                "nothing" => nothing,
                "revert" => revert,
                "signature" => signature,
                "definition" => definition,
                "file" => file,
                "fault" => fault,
                "repair" => repair,
                other => panic!("no rule named {other}"),
            }
        };
        self.rules.iter().map(|(name, weight)| Rule::new(name, *weight, apply(name))).collect()
    }

    fn invariants(&self) -> Vec<Invariant<Self>> {
        vec![Invariant::new_always_run("the step is written down", written)]
    }
}
