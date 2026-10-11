//! The events of a test run as sbt's test interface has them (`sbt.testing.Event`), whichever
//! runner made them: the JVM's (`runner.rs`, `runner/Runner.java`) or Scala.js's test bridge under
//! node (`jsrunner.rs`). Both runners hand their events over typed; the tally, the lines a failure
//! is reported by and the summary are made here, once for both platforms.

/// `sbt.testing.Status`, in its order: the bridge sends the ordinal.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Status {
    Success,
    Error,
    Failure,
    Skipped,
    Ignored,
    Canceled,
    Pending,
}

impl Status {
    pub const ALL: [Status; 7] = [Status::Success, Status::Error, Status::Failure, Status::Skipped, Status::Ignored, Status::Canceled, Status::Pending];

    pub fn of_ordinal(n: i64) -> Option<Status> {
        usize::try_from(n).ok().and_then(|i| Status::ALL.get(i).copied())
    }

    /// The name sbt prints (`Status.toString`).
    pub fn name(self) -> &'static str {
        match self {
            Status::Success => "Success",
            Status::Error => "Error",
            Status::Failure => "Failure",
            Status::Skipped => "Skipped",
            Status::Ignored => "Ignored",
            Status::Canceled => "Canceled",
            Status::Pending => "Pending",
        }
    }

    pub fn failed(self) -> bool {
        matches!(self, Status::Failure | Status::Error)
    }
}

/// `sbt.testing.Selector`: what of a suite an event is about.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Selector {
    Suite,
    Test(String),
    NestedSuite(String),
    NestedTest(String, String),
    Wildcard(String),
}

impl Selector {
    /// The selector as tests/scalajs-test's oracle records it.
    pub fn describe(&self) -> String {
        match self {
            Selector::Suite => "suite".to_string(),
            Selector::Test(t) => format!("test {}", t),
            Selector::NestedSuite(s) => format!("nestedSuite {}", s),
            Selector::NestedTest(s, t) => format!("nested {} {}", s, t),
            Selector::Wildcard(w) => format!("wildcard {}", w),
        }
    }
}

/// A frame of a throwable's stack: its class, and the line `printStackTrace` writes of it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Frame {
    pub class: String,
    pub text: String,
}

/// A throwable as the runner saw it: its `toString`, its frames and its cause.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Thrown {
    pub text: String,
    pub frames: Vec<Frame>,
    pub cause: Option<Box<Thrown>>,
}

impl Thrown {
    /// What `printStackTrace` writes, the frames a cause shares with what it caused said as
    /// `... n more`.
    pub fn trace(&self) -> String {
        let mut out = format!("{}\n", self.text);
        for f in &self.frames {
            out.push_str(&format!("\tat {}\n", f.text));
        }
        let mut enclosing = &self.frames;
        let mut cause = self.cause.as_deref();
        while let Some(c) = cause {
            let (mut m, mut n) = (c.frames.len(), enclosing.len());
            while m > 0 && n > 0 && c.frames[m - 1] == enclosing[n - 1] {
                m -= 1;
                n -= 1;
            }
            out.push_str(&format!("Caused by: {}\n", c.text));
            for f in &c.frames[..m] {
                out.push_str(&format!("\tat {}\n", f.text));
            }
            if m < c.frames.len() {
                out.push_str(&format!("\t... {} more\n", c.frames.len() - m));
            }
            enclosing = &c.frames;
            cause = c.cause.as_deref();
        }
        out
    }
}

/// An event of a task: the suite whose task it came from, the event's own name and selector,
/// its status, what it threw and its duration in milliseconds (-1 when the framework did not say);
/// `crashed` when no framework sent it, the runner making it of a task or a framework that threw.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Event {
    pub suite: String,
    pub name: String,
    pub selector: Selector,
    pub status: Status,
    pub thrown: Option<Thrown>,
    pub ms: i64,
    pub crashed: bool,
}

impl Event {
    /// A suite whose task or framework threw: an error of the suite, which nothing else reports,
    /// as sbt reports one, a `TestSelector` of the suite's name.
    pub fn crashed(suite: &str, thrown: Option<Thrown>) -> Event {
        Event { suite: suite.to_string(), name: suite.to_string(), selector: Selector::Test(suite.to_string()), status: Status::Error, thrown, ms: -1, crashed: true }
    }

    /// The event as a line of the inventory `teq test --events` writes, the oracle's
    /// (tests/scalajs-oracle.scala): the project, the event's name, its selector, its status and
    /// the first line of what it threw, tab-separated.
    pub fn line(&self, project: &str) -> String {
        let thrown = self.thrown.as_ref().and_then(|t| t.text.lines().next()).unwrap_or("");
        let clean = |s: &str| s.replace(['\t', '\n', '\r'], " ");
        format!("{}\t{}\t{}\t{}\t{}\n", clean(project), clean(&self.name), clean(&self.selector.describe()), self.status.name(), clean(thrown))
    }

    /// The test the event is of: its name and the test its selector names, said once.
    pub fn named(&self) -> String {
        let name = &self.name;
        let test = match &self.selector {
            Selector::Suite => "",
            Selector::Test(t) | Selector::NestedSuite(t) | Selector::Wildcard(t) => t.as_str(),
            Selector::NestedTest(s, t) => return named_test(name, &format!("{} {}", s, t)),
        };
        named_test(name, test)
    }
}

fn named_test(name: &str, test: &str) -> String {
    if test.is_empty() || test == name {
        name.to_string()
    } else if test.starts_with(name) {
        test.to_string()
    } else {
        format!("{} {}", name, test)
    }
}

/// The events of a run: each kept, in the order they came, and counted by status.
#[derive(Default, Debug)]
pub struct Tally {
    pub events: Vec<Event>,
    pub counts: [u64; 7],
}

impl Tally {
    /// The event counted and kept; the lines that report it, which a failure always has and
    /// any other event has when `verbose`.
    pub fn add(&mut self, event: Event, verbose: bool) -> String {
        self.counts[event.status as usize] += 1;
        let lines = report(&event, verbose);
        self.events.push(event);
        lines
    }

    pub fn count(&self, status: Status) -> u64 {
        self.counts[status as usize]
    }

    pub fn failed(&self) -> u64 {
        self.count(Status::Failure) + self.count(Status::Error)
    }

    pub fn passed(&self) -> u64 {
        self.count(Status::Success)
    }

    /// The counts after the passed and failed ones, those that are not zero: `, 1 ignored`.
    pub fn others(&self) -> String {
        let mut out = String::new();
        for (status, word) in [(Status::Ignored, "ignored"), (Status::Skipped, "skipped"), (Status::Pending, "pending"), (Status::Canceled, "canceled")] {
            let n = self.count(status);
            if n > 0 {
                out.push_str(&format!(", {} {}", n, word));
            }
        }
        out
    }
}

/// The lines of an event: a failure's `[fail] <test>: <first line>` with the throwable's frames in
/// the suite's own code (the first frame when none is), the cause the frames came from; with
/// `verbose` the whole trace, and a line for every other event, `[<status>] <test>`.
pub fn report(event: &Event, verbose: bool) -> String {
    let what = if event.crashed { format!("{} crashed", event.name) } else { event.named() };
    if !event.status.failed() {
        return if verbose { format!("[{}] {}\n", event.status.name().to_lowercase(), what) } else { String::new() };
    }
    let Some(e) = &event.thrown else { return format!("[fail] {}\n", what) };
    if verbose {
        return format!("[fail] {}: {}", what, e.trace());
    }
    let mut out = format!("[fail] {}: {}\n", what, e.text.lines().next().unwrap_or(""));
    let mut cause = e;
    while cause.frames.is_empty() {
        match &cause.cause {
            Some(c) => cause = c,
            None => break,
        }
    }
    let suite = &event.suite;
    let own: Vec<&Frame> = cause.frames.iter().filter(|f| f.class == *suite || f.class.starts_with(&format!("{}$", suite))).take(5).collect();
    for f in &own {
        out.push_str(&format!("[fail]     at {}\n", f.text));
    }
    if own.is_empty() {
        if let Some(f) = cause.frames.first() {
            out.push_str(&format!("[fail]     at {}\n", f.text));
        }
    }
    if !std::ptr::eq(cause, e) {
        out.push_str(&format!("[fail]   caused by {}\n", cause.text));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(class: &str, text: &str) -> Frame {
        Frame { class: class.to_string(), text: text.to_string() }
    }

    #[test]
    fn a_failure_is_reported_by_its_frames_in_the_suite() {
        let thrown = Thrown {
            text: "java.lang.AssertionError: no\nmore".to_string(),
            frames: vec![frame("munit.Assertions", "munit.Assertions.fail(Assertions.scala:1)"), frame("app.T", "app.T.f(T.scala:3)"), frame("app.T$x", "app.T$x.g(T.scala:4)")],
            cause: None,
        };
        let event = Event { suite: "app.T".to_string(), name: "app.T".to_string(), selector: Selector::Test("app.T.f".to_string()), status: Status::Failure, thrown: Some(thrown), ms: 3, crashed: false };
        assert_eq!(report(&event, false), "[fail] app.T.f: java.lang.AssertionError: no\n[fail]     at app.T.f(T.scala:3)\n[fail]     at app.T$x.g(T.scala:4)\n");
        let mut tally = Tally::default();
        tally.add(event, false);
        assert_eq!((tally.failed(), tally.passed(), tally.others()), (1, 0, String::new()));
    }

    #[test]
    fn a_cause_shares_its_last_frames() {
        let shared = frame("m.Main", "m.Main.main(Main.scala:9)");
        let cause = Thrown { text: "java.lang.IllegalStateException: inner".to_string(), frames: vec![frame("m.A", "m.A.a(A.scala:1)"), shared.clone()], cause: None };
        let outer = Thrown { text: "java.lang.RuntimeException: outer".to_string(), frames: vec![frame("m.B", "m.B.b(B.scala:2)"), shared], cause: Some(Box::new(cause)) };
        assert_eq!(
            outer.trace(),
            "java.lang.RuntimeException: outer\n\tat m.B.b(B.scala:2)\n\tat m.Main.main(Main.scala:9)\nCaused by: java.lang.IllegalStateException: inner\n\tat m.A.a(A.scala:1)\n\t... 1 more\n"
        );
    }

    #[test]
    fn every_status_is_counted_and_named() {
        let mut tally = Tally::default();
        for status in Status::ALL {
            let e = Event { suite: "s.S".to_string(), name: "s.S".to_string(), selector: Selector::Test("t".to_string()), status, thrown: None, ms: -1, crashed: false };
            let lines = tally.add(e, true);
            assert!(lines.starts_with(if status.failed() { "[fail] s.S t" } else { "[" }), "{}", lines);
        }
        assert_eq!((tally.failed(), tally.passed()), (2, 1));
        assert_eq!(tally.others(), ", 1 ignored, 1 skipped, 1 pending, 1 canceled");
        assert_eq!(Status::of_ordinal(4), Some(Status::Ignored));
        assert_eq!(Status::of_ordinal(7), None);
        assert_eq!(report(&Event::crashed("s.S", None), false), "[fail] s.S crashed\n");
    }
}
