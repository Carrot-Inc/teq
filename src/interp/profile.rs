//! The histogram behind `--profile` for macro expansions: where the interpreter's time and
//! steps go, per interpreted function, lambda, constructor and builtin, with the allocations
//! counted. Off, every hook is one branch on `Interp::prof` being `None`. Time is read from
//! the cycle counter (`ticks`), which costs a few nanoseconds where the clock costs tens; the
//! expander times each run with the clock as well, and the report converts.

use super::*;
use crate::tir::StrRef;

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum ProfKey {
    Fun(SymId),
    Lambda(TExprId),
    Ctor(ClassId),
    /// A builtin by its qualified name, or the text of a typer-made template.
    Builtin(std::sync::Arc<str>),
}

#[derive(Default, Clone, Copy)]
pub struct ProfRow {
    pub count: u64,
    pub self_ticks: u64,
    pub incl_ticks: u64,
    pub self_steps: u64,
    pub incl_steps: u64,
}

impl ProfRow {
    pub fn add(&mut self, other: &ProfRow) {
        self.count += other.count;
        self.self_ticks += other.self_ticks;
        self.incl_ticks += other.incl_ticks;
        self.self_steps += other.self_steps;
        self.incl_steps += other.incl_steps;
    }
}

#[derive(Default, Clone, Copy)]
pub struct Allocs {
    pub frames: u64,
    pub objects: u64,
    pub closures: u64,
    pub strings: u64,
}

impl Allocs {
    pub fn add(&mut self, other: &Allocs) {
        self.frames += other.frames;
        self.objects += other.objects;
        self.closures += other.closures;
        self.strings += other.strings;
    }
}

/// The processor's cycle counter, or the clock where there is none.
#[inline(always)]
pub fn ticks() -> u64 {
    #[cfg(target_arch = "aarch64")]
    {
        let t: u64;
        unsafe { std::arch::asm!("mrs {}, cntvct_el0", out(reg) t, options(nomem, nostack, preserves_flags)) };
        t
    }
    #[cfg(target_arch = "x86_64")]
    {
        unsafe { std::arch::x86_64::_rdtsc() }
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    {
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        START.get_or_init(std::time::Instant::now).elapsed().as_nanos() as u64
    }
}

struct Open {
    row: u32,
    start: u64,
    child_ticks: u64,
    steps_at_entry: u64,
    child_steps: u64,
}

const NONE: u32 = u32::MAX;

/// The histogram's state, kept from one expansion to the next (`InterpCaches`): the rows and
/// the indexes from a function, lambda body, class or template to its row are as large as the
/// program and are built once per build, not per expansion; a run reports the rows it touched
/// and zeroes them.
#[derive(Default)]
pub struct ProfState {
    open: Vec<Open>,
    keys: Vec<ProfKey>,
    rows: Vec<ProfRow>,
    touched: Vec<u32>,
    pub allocs: Allocs,
    fun_rows: Vec<u32>,
    lambda_rows: Vec<u32>,
    ctor_rows: Vec<u32>,
    template_rows: Vec<u32>,
    builtin_rows: FxMap<Rc<str>, u32>,
}

/// What one profiled run counted: its rows and allocations.
#[derive(Default)]
pub struct InterpProf {
    rows: Vec<(ProfKey, ProfRow)>,
    pub allocs: Allocs,
}

impl InterpProf {
    pub fn rows(&self) -> impl Iterator<Item = (&ProfKey, &ProfRow)> {
        self.rows.iter().map(|(k, r)| (k, r))
    }
}

impl ProfState {
    fn row_of(&mut self, key: ProfKey) -> u32 {
        self.keys.push(key);
        self.rows.push(ProfRow::default());
        (self.rows.len() - 1) as u32
    }

    fn cached(slots: &mut Vec<u32>, i: usize) -> Option<u32> {
        if slots.len() <= i {
            slots.resize(i + 1, NONE);
        }
        (slots[i] != NONE).then_some(slots[i])
    }

    fn take_run(&mut self) -> InterpProf {
        let mut rows = Vec::with_capacity(self.touched.len());
        for i in self.touched.drain(..) {
            let row = std::mem::take(&mut self.rows[i as usize]);
            rows.push((self.keys[i as usize].clone(), row));
        }
        self.open.clear();
        InterpProf { rows, allocs: std::mem::take(&mut self.allocs) }
    }
}

impl<'a, 't> Interp<'a, 't> {
    pub fn start_profile(&mut self) {
        let mut state = self.prof_state.take().unwrap_or_default();
        state.open.clear();
        state.allocs = Allocs::default();
        self.prof = Some(state);
    }

    pub fn take_profile(&mut self) -> Option<InterpProf> {
        let mut state = self.prof.take()?;
        let run = state.take_run();
        self.prof_state = Some(state);
        Some(run)
    }

    #[inline(always)]
    pub(super) fn prof_fun(&mut self, f: FunId) -> Option<usize> {
        if self.prof.is_none() {
            return None;
        }
        Some(self.prof_fun_slow(f))
    }

    fn prof_fun_slow(&mut self, f: FunId) -> usize {
        let p = self.prof.as_mut().unwrap();
        let row = match ProfState::cached(&mut p.fun_rows, f.idx()) {
            Some(r) => r,
            None => {
                let sym = self.typer.prog.funs[f.idx()].sym;
                let r = p.row_of(ProfKey::Fun(sym));
                p.fun_rows[f.idx()] = r;
                r
            }
        };
        self.prof_enter(row)
    }

    #[inline(always)]
    pub(super) fn prof_lambda(&mut self, body: TExprId) -> Option<usize> {
        if self.prof.is_none() {
            return None;
        }
        Some(self.prof_lambda_slow(body))
    }

    fn prof_lambda_slow(&mut self, body: TExprId) -> usize {
        let p = self.prof.as_mut().unwrap();
        let row = match ProfState::cached(&mut p.lambda_rows, body.idx()) {
            Some(r) => r,
            None => {
                let r = p.row_of(ProfKey::Lambda(body));
                p.lambda_rows[body.idx()] = r;
                r
            }
        };
        self.prof_enter(row)
    }

    #[inline(always)]
    pub(super) fn prof_ctor(&mut self, c: ClassId) -> Option<usize> {
        if self.prof.is_none() {
            return None;
        }
        Some(self.prof_ctor_slow(c))
    }

    fn prof_ctor_slow(&mut self, c: ClassId) -> usize {
        let p = self.prof.as_mut().unwrap();
        let row = match ProfState::cached(&mut p.ctor_rows, c.idx()) {
            Some(r) => r,
            None => {
                let r = p.row_of(ProfKey::Ctor(c));
                p.ctor_rows[c.idx()] = r;
                r
            }
        };
        self.prof_enter(row)
    }

    #[inline(always)]
    pub(super) fn prof_builtin(&mut self, name: &Rc<str>) -> Option<usize> {
        if self.prof.is_none() {
            return None;
        }
        Some(self.prof_builtin_slow(name))
    }

    fn prof_builtin_slow(&mut self, name: &Rc<str>) -> usize {
        let p = self.prof.as_mut().unwrap();
        let row = match p.builtin_rows.get(name) {
            Some(&r) => r,
            None => {
                let r = p.row_of(ProfKey::Builtin(std::sync::Arc::from(&**name)));
                p.builtin_rows.insert(name.clone(), r);
                r
            }
        };
        self.prof_enter(row)
    }

    /// The builtin behind a template, for the histogram: the def's qualified name, or the
    /// template's text where the typer made it.
    #[inline(always)]
    pub(super) fn prof_template(&mut self, s: StrRef) -> Option<usize> {
        if self.prof.is_none() {
            return None;
        }
        Some(self.prof_template_slow(s))
    }

    fn prof_template_slow(&mut self, s: StrRef) -> usize {
        let p = self.prof.as_mut().unwrap();
        let row = match ProfState::cached(&mut p.template_rows, s.idx()) {
            Some(r) => r,
            None => {
                let name: Rc<str> = match self.typer.prog.template_syms.get(&s).copied() {
                    Some(sym) => self.qualified_name(sym),
                    None => Rc::from(self.typer.prog.strings[s.idx()].as_str()),
                };
                let p = self.prof.as_mut().unwrap();
                let r = match p.builtin_rows.get(&name) {
                    Some(&r) => r,
                    None => {
                        let r = p.row_of(ProfKey::Builtin(std::sync::Arc::from(&*name)));
                        p.builtin_rows.insert(name, r);
                        r
                    }
                };
                p.template_rows[s.idx()] = r;
                r
            }
        };
        self.prof_enter(row)
    }

    fn prof_enter(&mut self, row: u32) -> usize {
        let steps = self.steps;
        let p = self.prof.as_mut().unwrap();
        p.open.push(Open { row, start: ticks(), child_ticks: 0, steps_at_entry: steps, child_steps: 0 });
        p.open.len() - 1
    }

    #[inline(always)]
    pub(super) fn prof_exit(&mut self, open: Option<usize>) {
        if let Some(i) = open {
            self.prof_exit_slow(i);
        }
    }

    fn prof_exit_slow(&mut self, i: usize) {
        let steps = self.steps;
        let Some(p) = self.prof.as_mut() else { return };
        if p.open.len() != i + 1 {
            return;
        }
        let o = p.open.pop().unwrap();
        let incl = ticks().wrapping_sub(o.start);
        let incl_steps = o.steps_at_entry.saturating_sub(steps);
        if p.rows[o.row as usize].count == 0 {
            p.touched.push(o.row);
        }
        let r = &mut p.rows[o.row as usize];
        r.count += 1;
        r.incl_ticks += incl;
        r.self_ticks += incl.saturating_sub(o.child_ticks);
        r.incl_steps += incl_steps;
        r.self_steps += incl_steps.saturating_sub(o.child_steps);
        if let Some(parent) = p.open.last_mut() {
            parent.child_ticks += incl;
            parent.child_steps += incl_steps;
        }
    }

    #[inline(always)]
    pub(super) fn prof_alloc(&mut self, f: impl FnOnce(&mut Allocs)) {
        if let Some(p) = self.prof.as_mut() {
            f(&mut p.allocs);
        }
    }
}
