//! The merge's threads: as many as the build had workers, the
//! calling thread one of them, started once for the merge and given one part at a time. A part
//! is a function of the thread's number, run by every thread at once; `run` returns when every
//! thread has finished it, so a part may borrow what its caller holds. With one thread a part is
//! a call.

use std::sync::{Condvar, Mutex};

/// A part handed to the crew: the function, with its borrow's lifetime erased (`run` waits for
/// every thread before the borrow ends), and the part's number.
#[derive(Clone, Copy)]
struct Part {
    job: *const (dyn Fn(usize) + Sync),
    number: u64,
}

unsafe impl Send for Part {}

struct State {
    part: Option<Part>,
    /// The threads still running the current part, the calling thread's share excluded.
    running: usize,
    /// A crew thread's part panicked: the caller panics after the part.
    panicked: bool,
    stop: bool,
}

pub struct Crew {
    threads: usize,
    state: Mutex<State>,
    begun: Condvar,
    done: Condvar,
    /// What the merge gives back (`give_back`), freed on one thread when the crew ends.
    garbage: Mutex<Vec<Box<dyn Send>>>,
}

impl Drop for Crew {
    fn drop(&mut self) {
        let garbage = std::mem::take(self.garbage.get_mut().unwrap_or_else(|e| e.into_inner()));
        if !garbage.is_empty() {
            give_back(garbage);
        }
    }
}

impl Crew {
    /// Runs `f` with a crew of `threads` threads (`f`'s own one of them) for its parts.
    pub fn with<R>(threads: usize, f: impl FnOnce(&Crew) -> R) -> R {
        let crew = Crew { threads: threads.max(1), state: Mutex::new(State { part: None, running: 0, panicked: false, stop: false }), begun: Condvar::new(), done: Condvar::new(), garbage: Mutex::new(Vec::new()) };
        if crew.threads == 1 {
            return f(&crew);
        }
        std::thread::scope(|s| {
            let handles: Vec<_> = (1..crew.threads)
                .map(|k| {
                    let crew = &crew;
                    std::thread::Builder::new()
                        .name(format!("teq-merge-{}", k))
                        .spawn_scoped(s, move || {
                            crate::alloc::enter();
                            crew.serve(k)
                        })
                        .expect("cannot start a merge thread")
                })
                .collect();
            // The threads stop when `f` returns or unwinds, so that the scope's join ends.
            struct Stop<'c>(&'c Crew);
            impl Drop for Stop<'_> {
                fn drop(&mut self) {
                    self.0.lock().stop = true;
                    self.0.begun.notify_all();
                }
            }
            let r = {
                let _stop = Stop(&crew);
                f(&crew)
            };
            // Joined through their handles, the threads' own handles are freed here, not on the
            // ending threads, whose lists they would be lost with (`alloc.rs`).
            for h in handles {
                let _ = h.join();
            }
            r
        })
    }

    /// How many threads run each part.
    pub fn threads(&self) -> usize {
        self.threads
    }

    /// Keeps `v` to be dropped when the crew ends, on a thread of its own (`give_back`), with
    /// whatever else the merge gives back.
    pub fn give_back<T: Send + 'static>(&self, v: T) {
        self.garbage.lock().unwrap_or_else(|e| e.into_inner()).push(Box::new(v));
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// A crew thread: runs each part as it is handed over, until the crew stops.
    fn serve(&self, k: usize) {
        let mut seen = 0u64;
        loop {
            let part = {
                let mut s = self.lock();
                loop {
                    if s.stop {
                        return;
                    }
                    match s.part {
                        Some(p) if p.number != seen => break p,
                        _ => s = self.begun.wait(s).unwrap_or_else(|e| e.into_inner()),
                    }
                }
            };
            seen = part.number;
            // SAFETY: `run` keeps the part's function alive until every thread reported it done.
            let job = unsafe { &*part.job };
            let ok = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| job(k))).is_ok();
            let mut s = self.lock();
            s.panicked |= !ok;
            s.running -= 1;
            if s.running == 0 {
                self.done.notify_all();
            }
        }
    }

    /// Runs `job(k)` on every thread `k` of the crew at once, the caller's being 0, and returns
    /// when all are done.
    pub fn run(&self, job: &(dyn Fn(usize) + Sync)) {
        if self.threads == 1 {
            return job(0);
        }
        // SAFETY: the lifetime is erased for the crew threads' copies only, and this call does not
        // return before every one of them has finished with it.
        let erased: *const (dyn Fn(usize) + Sync) = unsafe { std::mem::transmute::<&(dyn Fn(usize) + Sync), &'static (dyn Fn(usize) + Sync)>(job) };
        {
            let mut s = self.lock();
            let number = s.part.map_or(1, |p| p.number + 1);
            s.part = Some(Part { job: erased, number });
            s.running = self.threads - 1;
        }
        self.begun.notify_all();
        let mine = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| job(0)));
        let mut s = self.lock();
        while s.running > 0 {
            s = self.done.wait(s).unwrap_or_else(|e| e.into_inner());
        }
        let panicked = std::mem::take(&mut s.panicked);
        drop(s);
        if let Err(e) = mine {
            std::panic::resume_unwind(e);
        }
        assert!(!panicked, "a merge thread panicked");
    }

    /// `run` over `n` tasks, thread `k` taking the tasks `k`, `k + threads`, and so on.
    pub fn each(&self, n: usize, task: &(dyn Fn(usize) + Sync)) {
        let threads = self.threads;
        self.run(&|k| {
            let mut i = k;
            while i < n {
                task(i);
                i += threads;
            }
        });
    }

    /// The `k`th of the crew's contiguous shares of `range`, as even as whole items allow.
    pub fn share(&self, k: usize, range: std::ops::Range<usize>) -> std::ops::Range<usize> {
        share(k, self.threads, range)
    }
}

/// The `k`th of `n` contiguous shares of `range`.
pub fn share(k: usize, n: usize, range: std::ops::Range<usize>) -> std::ops::Range<usize> {
    let len = range.end - range.start;
    let at = |i: usize| range.start + len * i / n;
    at(k)..at(k + 1)
}

/// Drops `v` on a thread of its own, off the merge's main thread: the
/// overlays given back, the indexes and memos the join retires, the record versions published
/// copies replaced. With the memory measured (`TEQ_WORKERS_MEMORY=1`) dropped here, so that the
/// merge's barriers count what it gave back.
pub fn give_back<T: Send + 'static>(v: T) {
    if crate::measure::memory_wanted() {
        return drop(v);
    }
    crate::alloc::spawn(move || drop(v));
}

/// A value no thread but its holder refers to any more, given back (`give_back`) though its type
/// holds what is not `Send` in general (raw pointers, the overlays' cells).
pub struct Unshared<T>(pub T);

unsafe impl<T> Send for Unshared<T> {}

/// A pointer handed to the crew's threads, each of which writes a part of what it points to that
/// no other thread writes.
#[derive(Clone, Copy)]
pub struct Shared<T>(pub *mut T);

unsafe impl<T> Send for Shared<T> {}
unsafe impl<T> Sync for Shared<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn every_thread_runs_every_part_and_the_borrow_outlives_it() {
        for threads in [1, 2, 5] {
            Crew::with(threads, |crew| {
                for round in 0..20 {
                    let hits: Vec<AtomicUsize> = (0..threads).map(|_| AtomicUsize::new(0)).collect();
                    crew.run(&|k| {
                        hits[k].fetch_add(round + 1, Ordering::Relaxed);
                    });
                    assert!(hits.iter().all(|h| h.load(Ordering::Relaxed) == round + 1));
                }
                let sum = AtomicUsize::new(0);
                crew.each(100, &|i| {
                    sum.fetch_add(i, Ordering::Relaxed);
                });
                assert_eq!(sum.load(Ordering::Relaxed), 4950);
            });
        }
    }

    #[test]
    fn the_shares_cover_the_range_once() {
        for n in 1..7 {
            let mut next = 3;
            for k in 0..n {
                let r = share(k, n, 3..20);
                assert_eq!(r.start, next);
                next = r.end;
            }
            assert_eq!(next, 20);
        }
    }

    #[test]
    fn a_crew_threads_panic_reaches_the_caller() {
        let caught = std::panic::catch_unwind(|| {
            Crew::with(3, |crew| {
                crew.run(&|k| {
                    if k == 2 {
                        panic!("the part");
                    }
                })
            })
        });
        assert!(caught.is_err());
    }
}
