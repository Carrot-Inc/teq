//! The test runner of `teq test` on Scala.js (docs/TOOLING.md, "Tests"): sbt-scalajs's side of
//! Scala.js's test bridge, as sbt-scalajs 1.22.0 and its JS environments do it. The test link's
//! entry point starts the bridge (std/scalajs/test_bridge.scala, Scala.js's `test-bridge`), which
//! finds the frameworks by their names through reflection, makes their runners, runs their tasks
//! through the continuation overload of `Task.execute` and sends their events and log lines back
//! typed. What `TestAdapter`, `RunnerAdapter` and `TaskAdapter` do on sbt's side (`RPCCore`,
//! `RunMuxRPC`, the serializers) is done here, over the com channel `NodeJSEnv` and
//! `JSDOMNodeJSEnv` set up (`ComRun`: a loopback socket, each message its length in UTF-16 units,
//! then the units big-endian, each a byte of the serialized message): one node per run, given its
//! program on stdin as the environments give it, which loads the link as the module kind says;
//! one controller runner per framework, its tasks executed one at a time and the tasks a task
//! returns after it, then the runner's `done`. A run completes when every call has had its reply
//! and node, its channel closed, exits with 0: a node that ends early, a reply that is an error
//! or a status other than 0 fails it.

use super::events::{Event, Frame, Selector, Status, Tally, Thrown};
use super::runner::{Fingerprint, Kind};
use std::collections::BTreeMap;
use std::io::{BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How long node has to connect to the channel once started, and to exit once it is closed.
const CONNECT_BOUND: Duration = Duration::from_secs(60);
const EXIT_BOUND: Duration = Duration::from_secs(10);

/// The link as the environment loads it (`org.scalajs.jsenv.Input`).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Input {
    /// Read and run in node's global scope (`vm.runInThisContext`): `NoModule`.
    Script(PathBuf),
    /// `require`d: `CommonJSModule`.
    CommonJs(PathBuf),
    /// `import`ed: `ESModule`.
    EsModule(PathBuf),
}

/// `NodeJSEnv.SourceMap`: whether node installs `source-map-support` before the program.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SourceMap {
    Disable,
    EnableIfAvailable,
    Enable,
}

/// The JS environment of a run: node's executable, its arguments and variables, the directory it
/// runs in, and for `JSDOMNodeJSEnv` the package root jsdom is required from.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Env {
    pub executable: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub cwd: PathBuf,
    pub source_map: SourceMap,
    pub jsdom: Option<PathBuf>,
}

/// A JavaScript string literal of the text.
fn js_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c == '\u{2028}' || c == '\u{2029}' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A file's `file:` URL, as `File.toURI` gives it.
fn file_url(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    let mut out = String::from(if text.starts_with('/') { "file://" } else { "file:///" });
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~:".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

/// The com channel's setup, `ComRun.setupFile`'s text: a socket to the runner's port, the
/// messages framed as above, `scalajsCom` for the bridge, and the process ended with the socket.
fn com_setup(port: u16) -> String {
    format!(
        r#"(function() {{
  var socket = require('net').connect({port}, '127.0.0.1');
  var inBuffer = Buffer.alloc(0);
  var inMessages = [];
  var onMessage = null;
  socket.on('data', function(data) {{
    inBuffer = Buffer.concat([inBuffer, data]);
    while (inBuffer.length >= 4) {{
      var msgLen = inBuffer.readInt32BE(0);
      var byteLen = 4 + msgLen * 2;
      if (inBuffer.length < byteLen) return;
      var res = "";
      for (var i = 0; i < msgLen; ++i)
        res += String.fromCharCode(inBuffer.readInt16BE(4 + i * 2));
      inBuffer = inBuffer.slice(byteLen);
      if (inMessages !== null) inMessages.push(res);
      else onMessage(res);
    }}
  }});
  socket.on('error', function(err) {{
    console.error("Scala.js Com failed: " + err);
    process.exit(-1);
  }});
  socket.on('close', function() {{ process.exit(0); }});
  global.scalajsCom = {{
    init: function(onMsg) {{
      if (onMessage !== null) throw new Error("Com already initialized");
      onMessage = onMsg;
      process.nextTick(function() {{
        for (var i = 0; i < inMessages.length; ++i)
          onMessage(inMessages[i]);
        inMessages = null;
      }});
    }},
    send: function(msg) {{
      var len = msg.length;
      var buf = Buffer.allocUnsafe(4 + len * 2);
      buf.writeInt32BE(len, 0);
      for (var i = 0; i < len; ++i)
        buf.writeUInt16BE(msg.charCodeAt(i), 4 + i * 2);
      socket.write(buf);
    }}
  }};
}}).call(this);
"#
    )
}

/// `NodeJSEnv.write`'s script of a file: read and run in node's global scope.
fn run_script(path: &Path) -> String {
    let p = js_string(&path.to_string_lossy());
    format!("require('vm').runInThisContext(require('fs').readFileSync({p}, {{ encoding: \"utf-8\" }}), {{ filename: {p}, displayErrors: true }});\n")
}

/// The program node reads on stdin: the source-map support the environment asks for, the com
/// channel, then the link as `NodeJSEnv.write` loads its inputs (a chain of imports once one is an
/// ES module), or inside jsdom's window as `JSDOMNodeJSEnv.codeWithJSDOMContext` does, jsdom
/// required from the recorded package root.
pub fn program(port: u16, env: &Env, input: &Input) -> String {
    let mut out = String::new();
    match (env.jsdom.is_some(), env.source_map) {
        (false, SourceMap::EnableIfAvailable) => out.push_str("try { require('source-map-support').install(); } catch (e) {};\n"),
        (false, SourceMap::Enable) => out.push_str("require('source-map-support').install();\n"),
        _ => {}
    }
    out.push_str(&com_setup(port));
    if let Some(root) = &env.jsdom {
        let Input::Script(script) = input else { unreachable!("jsdom runs a script alone, which the caller checked") };
        let from = js_string(&root.join("package.json").to_string_lossy());
        let src = js_string(&file_url(script));
        out.push_str(&format!(
            r#"(function () {{
  var jsdom = require('module').createRequire({from})("jsdom");
  var virtualConsole = new jsdom.VirtualConsole();
  if (typeof virtualConsole.forwardTo === 'function')
    virtualConsole.forwardTo(console, {{ omitJSDOMErrors: true }})
  else
    virtualConsole.sendTo(console, {{ omitJSDOMErrors: true }});
  virtualConsole.on("jsdomError", function (error) {{
    var isWithinReactsInvokeGuardedCallbackDevHack_issue42 =
      new Error("").stack.indexOf("invokeGuardedCallbackDev") >= 0;
    if (isWithinReactsInvokeGuardedCallbackDevHack_issue42)
      return;
    try {{
      if (error.detail && error.detail.stack) {{
        console.error("" + error.detail);
        console.error(error.detail.stack);
      }} else {{
        console.error(error);
      }}
    }} finally {{
      process.exit(1);
    }}
  }});
  var dom = new jsdom.JSDOM("", {{
    virtualConsole: virtualConsole,
    url: "http://localhost/",
    resources: "usable",
    runScripts: "dangerously"
  }});
  var window = dom.window;
  window["scalajsCom"] = global.scalajsCom;
  process.on('exit', function () {{ window.close(); }});
  var script = window.document.createElement("script");
  script.src = {src};
  window.document.body.appendChild(script);
}})();
"#
        ));
        return out;
    }
    match input {
        Input::Script(path) => out.push_str(&run_script(path)),
        Input::CommonJs(path) => out.push_str(&format!("require({});\n", js_string(&path.to_string_lossy()))),
        Input::EsModule(path) => out.push_str(&format!(
            "Promise.resolve().\n  then(() => import({})).catch(e => {{\n  console.error(e);\n  process.exit(1);\n}});\n",
            js_string(&file_url(path))
        )),
    }
    out
}

// ---- the serializers of org.scalajs.testing.common ----

/// A message under construction: a byte stream of `DataOutputStream`'s encodings.
struct Out(Vec<u8>);

impl Out {
    fn new() -> Out {
        Out(Vec::new())
    }
    fn byte(&mut self, b: u8) -> &mut Out {
        self.0.push(b);
        self
    }
    fn bool(&mut self, b: bool) -> &mut Out {
        self.byte(u8::from(b))
    }
    fn int(&mut self, n: i32) -> &mut Out {
        self.0.extend_from_slice(&n.to_be_bytes());
        self
    }
    fn long(&mut self, n: i64) -> &mut Out {
        self.0.extend_from_slice(&n.to_be_bytes());
        self
    }
    /// `StringSerializer`: the length in UTF-16 units, then each unit in one to three bytes.
    fn string(&mut self, s: &str) -> &mut Out {
        let units: Vec<u16> = s.encode_utf16().collect();
        self.int(units.len() as i32);
        for c in units {
            if (0x01..=0x7f).contains(&c) {
                self.0.push(c as u8);
            } else if c < 0x800 {
                self.0.push(((c >> 6) | 0xc0) as u8);
                self.0.push(((c & 0x3f) | 0x80) as u8);
            } else {
                self.0.push(((c >> 12) | 0xe0) as u8);
                self.0.push((((c >> 6) & 0x3f) | 0x80) as u8);
                self.0.push(((c & 0x3f) | 0x80) as u8);
            }
        }
        self
    }
    fn strings(&mut self, items: &[String]) -> &mut Out {
        self.int(items.len() as i32);
        for s in items {
            self.string(s);
        }
        self
    }
    fn fingerprint(&mut self, f: &Fingerprint) -> &mut Out {
        match f.kind {
            Kind::Annotated => self.byte(1).bool(f.module).string(&f.name),
            Kind::Subclass => self.byte(2).bool(f.module).string(&f.name).bool(f.no_args),
        }
    }
    fn task_def(&mut self, d: &TaskDef) -> &mut Out {
        self.string(&d.name).fingerprint(&d.fingerprint).bool(false);
        // One `SuiteSelector`.
        self.int(1).byte(1)
    }
    fn task_info(&mut self, t: &TaskInfo) -> &mut Out {
        self.string(&t.serialized);
        self.0.extend_from_slice(&t.def_bytes);
        self.strings(&t.tags)
    }
}

/// A received message: the bytes and where reading is.
struct In<'a> {
    bytes: &'a [u8],
    at: usize,
}

type Got<T> = Result<T, String>;

impl<'a> In<'a> {
    fn take(&mut self, n: usize) -> Got<&'a [u8]> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.bytes.len()).ok_or("a message of the test bridge ends early")?;
        let s = &self.bytes[self.at..end];
        self.at = end;
        Ok(s)
    }
    fn byte(&mut self) -> Got<u8> {
        Ok(self.take(1)?[0])
    }
    fn bool(&mut self) -> Got<bool> {
        Ok(self.byte()? != 0)
    }
    fn int(&mut self) -> Got<i32> {
        Ok(i32::from_be_bytes(self.take(4)?.try_into().expect("four")))
    }
    fn long(&mut self) -> Got<i64> {
        Ok(i64::from_be_bytes(self.take(8)?.try_into().expect("eight")))
    }
    fn count(&mut self) -> Got<usize> {
        usize::try_from(self.int()?).map_err(|_| "a negative count in a message of the test bridge".to_string())
    }
    fn string(&mut self) -> Got<String> {
        let n = self.count()?;
        let mut units = Vec::with_capacity(n.min(1 << 20));
        for _ in 0..n {
            let a = u16::from(self.byte()?);
            let unit = if a & 0x80 == 0 {
                a
            } else if a & 0xe0 == 0xc0 {
                ((a & 0x1f) << 6) | (u16::from(self.byte()?) & 0x3f)
            } else {
                let (b, c) = (u16::from(self.byte()?), u16::from(self.byte()?));
                ((a & 0x0f) << 12) | ((b & 0x3f) << 6) | (c & 0x3f)
            };
            units.push(unit);
        }
        Ok(String::from_utf16_lossy(&units))
    }
    fn option<T>(&mut self, read: impl FnOnce(&mut In<'a>) -> Got<T>) -> Got<Option<T>> {
        if self.bool()? {
            read(self).map(Some)
        } else {
            Ok(None)
        }
    }
    fn list<T>(&mut self, mut read: impl FnMut(&mut In<'a>) -> Got<T>) -> Got<Vec<T>> {
        let n = self.count()?;
        (0..n).map(|_| read(self)).collect()
    }
    fn fingerprint(&mut self) -> Got<(Kind, bool, String, bool)> {
        match self.byte()? {
            1 => Ok((Kind::Annotated, self.bool()?, self.string()?, false)),
            2 => Ok((Kind::Subclass, self.bool()?, self.string()?, self.bool()?)),
            t => Err(format!("a fingerprint of unknown type {} from the test bridge", t)),
        }
    }
    fn selector(&mut self) -> Got<Selector> {
        Ok(match self.byte()? {
            1 => Selector::Suite,
            2 => Selector::Test(self.string()?),
            3 => Selector::NestedSuite(self.string()?),
            4 => Selector::NestedTest(self.string()?, self.string()?),
            5 => Selector::Wildcard(self.string()?),
            t => return Err(format!("a selector of unknown type {} from the test bridge", t)),
        })
    }
    /// `ThrowableSerializer`: the message, the `toString`, the frames and the cause; a frame's
    /// text as `StackTraceElement.toString` writes it.
    fn throwable(&mut self) -> Got<Thrown> {
        let _message = self.option(In::string)?;
        let text = self.string()?;
        let frames = self.list(|i| {
            let (class, method, file, line) = (i.string()?, i.string()?, i.option(In::string)?, i.int()?);
            let at = match (file, line) {
                (Some(f), l) if l >= 0 => format!("{}:{}", f, l),
                (Some(f), _) => f,
                (None, _) => "Unknown Source".to_string(),
            };
            Ok(Frame { text: format!("{}.{}({})", class, method, at), class })
        })?;
        let cause = self.option(In::throwable)?.map(Box::new);
        Ok(Thrown { text, frames, cause })
    }
    /// A task's definition, its bytes kept to be sent back as they came.
    fn task_info(&mut self) -> Got<TaskInfo> {
        let serialized = self.string()?;
        let start = self.at;
        let name = self.string()?;
        let (kind, module, fp_name, no_args) = self.fingerprint()?;
        let _explicit = self.bool()?;
        let _selectors = self.list(In::selector)?;
        let def_bytes = self.bytes[start..self.at].to_vec();
        let tags = self.list(In::string)?;
        let fingerprint = Fingerprint { framework: 0, index: 0, kind, module, name: fp_name, no_args };
        Ok(TaskInfo { serialized, name, def_bytes, tags, fingerprint })
    }
}

/// A suite to run: its name and the fingerprint it was discovered by.
pub struct TaskDef {
    pub name: String,
    pub fingerprint: Fingerprint,
}

/// A task the bridge made (`TaskInfo`): the task as the framework serialized it, its definition's
/// bytes and its tags.
struct TaskInfo {
    serialized: String,
    name: String,
    def_bytes: Vec<u8>,
    tags: Vec<String>,
    #[allow(dead_code)]
    fingerprint: Fingerprint,
}

/// A framework as the bridge found it (`FrameworkInfo`, its display name left): its
/// implementation's class and its fingerprints.
pub struct FrameworkInfo {
    pub implementation: String,
    pub fingerprints: Vec<(Kind, bool, String, bool)>,
}

// The bridge's endpoints (`JSEndpoints`, `JVMEndpoints`) and the replies (`RPCCore`).
const REPLY_OK: u8 = 0;
const REPLY_ERR: u8 = 1;
const DETECT_FRAMEWORKS: u8 = 2;
const CREATE_CONTROLLER_RUNNER: u8 = 3;
const TASKS: u8 = 7;
const EXECUTE: u8 = 8;
const DONE: u8 = 9;
const EVENT: u8 = 4;
const LOG_ERROR: u8 = 5;
const LOG_TRACE: u8 = 9;

/// Where a run's lines go, from the runner's thread and from those reading node's output alike.
pub type Output = Arc<dyn Fn(&str) + Send + Sync>;

/// A node running the test link, its channel connected.
pub struct JsRun {
    child: Arc<Mutex<Child>>,
    stream: BufReader<TcpStream>,
    writer: TcpStream,
    next_id: i64,
    out: Output,
    readers: Vec<std::thread::JoinHandle<()>>,
    verbose: bool,
    /// The suite whose task is executing, which its events are of.
    suite: String,
}

/// Node's output, a line at a time as it comes, to `out`.
fn forward(mut from: impl Read + Send + 'static, out: Output) -> std::thread::JoinHandle<()> {
    crate::alloc::spawn(move || {
        let mut buf = [0u8; 8192];
        let mut pending: Vec<u8> = Vec::new();
        loop {
            let n = match from.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            pending.extend_from_slice(&buf[..n]);
            if let Some(end) = pending.iter().rposition(|&b| b == b'\n') {
                let rest = pending.split_off(end + 1);
                out(&String::from_utf8_lossy(&pending));
                pending = rest;
            }
        }
        if !pending.is_empty() {
            let mut text = String::from_utf8_lossy(&pending).into_owned();
            text.push('\n');
            out(&text);
        }
    })
}

impl JsRun {
    /// Starts node in the environment's directory with the client's variables, the environment's
    /// and `NODE_MODULE_CONTEXTS=0` (the environments' own), the program on its stdin, and waits
    /// for it to connect to the channel.
    pub fn start(env: &Env, input: &Input, environment: &BTreeMap<String, String>, verbose: bool, out: Output) -> Result<JsRun, String> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| format!("cannot listen on the loopback interface: {}", e))?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let mut child = Command::new(&env.executable)
            .args(&env.args)
            .current_dir(&env.cwd)
            .env_clear()
            .envs(environment)
            .env("NODE_MODULE_CONTEXTS", "0")
            .envs(&env.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("cannot start {}: {}", env.executable, e))?;
        let mut stdin = child.stdin.take().expect("piped");
        let readers = vec![forward(child.stdout.take().expect("piped"), out.clone()), forward(child.stderr.take().expect("piped"), out.clone())];
        let child = Arc::new(Mutex::new(child));
        let fail = |child: &Arc<Mutex<Child>>, why: String| {
            let mut c = child.lock().unwrap_or_else(|e| e.into_inner());
            let _ = c.kill();
            let _ = c.wait();
            why
        };
        if let Err(e) = stdin.write_all(program(port, env, input).as_bytes()) {
            return Err(fail(&child, format!("cannot give {} its program: {}", env.executable, e)));
        }
        drop(stdin);
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let until = Instant::now() + CONNECT_BOUND;
        let stream = loop {
            match listener.accept() {
                Ok((s, _)) => break s,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if let Ok(Some(status)) = child.lock().unwrap_or_else(|e| e.into_inner()).try_wait() {
                        return Err(format!("{} ended ({}) before the test bridge started", env.executable, status));
                    }
                    if Instant::now() > until {
                        return Err(fail(&child, format!("{} did not start the test bridge in {} s", env.executable, CONNECT_BOUND.as_secs())));
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(e) => return Err(fail(&child, format!("the test bridge's channel: {}", e))),
            }
        };
        stream.set_nonblocking(false).map_err(|e| e.to_string())?;
        let writer = stream.try_clone().map_err(|e| e.to_string())?;
        Ok(JsRun { child, stream: BufReader::new(stream), writer, next_id: 0, out, readers, verbose, suite: String::new() })
    }

    /// The process, for a watcher to kill when the client that asked for the run is gone.
    pub fn process(&self) -> Arc<Mutex<Child>> {
        self.child.clone()
    }

    fn send(&mut self, message: &[u8]) -> Result<(), String> {
        let mut frame = Vec::with_capacity(4 + message.len() * 2);
        frame.extend_from_slice(&(message.len() as i32).to_be_bytes());
        for &b in message {
            frame.extend_from_slice(&u16::from(b).to_be_bytes());
        }
        self.writer.write_all(&frame).map_err(|_| self.ended("before its call was sent"))
    }

    /// The next message, or why there is none: node closed the channel.
    fn receive(&mut self) -> Result<Vec<u8>, String> {
        let mut len = [0u8; 4];
        self.stream.read_exact(&mut len).map_err(|_| self.ended("before the run completed"))?;
        let n = usize::try_from(i32::from_be_bytes(len)).map_err(|_| "a message of negative length from the test bridge".to_string())?;
        let mut units = vec![0u8; n * 2];
        self.stream.read_exact(&mut units).map_err(|_| self.ended("in the middle of a message"))?;
        Ok(units.chunks(2).map(|u| u[1]).collect())
    }

    fn ended(&self, when: &str) -> String {
        let status = self.wait_exit(EXIT_BOUND);
        format!("node ended {}{}", when, status.map_or(String::new(), |s| format!(" ({})", s)))
    }

    /// node's exit status, waited for at most `bound`, then killed.
    fn wait_exit(&self, bound: Duration) -> Option<std::process::ExitStatus> {
        let until = Instant::now() + bound;
        loop {
            let mut c = self.child.lock().unwrap_or_else(|e| e.into_inner());
            match c.try_wait() {
                Ok(Some(s)) => return Some(s),
                Ok(None) if Instant::now() < until => {}
                _ => {
                    let _ = c.kill();
                    return c.wait().ok();
                }
            }
            drop(c);
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// A call of an endpoint and its reply, the events and log lines that come before it
    /// counted into `tally` and written; a reply that is an error is its throwable.
    fn call(&mut self, op: u8, payload: &Out, tally: &mut Tally) -> Result<Result<Vec<u8>, Thrown>, String> {
        self.next_id += 1;
        let id = self.next_id;
        let mut message = Out::new();
        message.byte(op).long(id);
        message.0.extend_from_slice(&payload.0);
        self.send(&message.0)?;
        loop {
            let bytes = self.receive()?;
            let mut i = In { bytes: &bytes, at: 0 };
            match i.byte()? {
                r @ (REPLY_OK | REPLY_ERR) => {
                    if i.long()? != id {
                        continue;
                    }
                    // What failed in the bridge, as sbt-scalajs's `RPCCore` throws it: an
                    // `RPCException` caused by it.
                    if r == REPLY_ERR {
                        let cause = i.throwable()?;
                        return Ok(Err(Thrown { text: format!("org.scalajs.testing.common.RPCCore$RPCException: {}", cause.text), frames: Vec::new(), cause: Some(Box::new(cause)) }));
                    }
                    return Ok(Ok(bytes[i.at..].to_vec()));
                }
                EVENT => {
                    let _run = i.int()?;
                    let name = i.string()?;
                    let _fingerprint = i.fingerprint()?;
                    let selector = i.selector()?;
                    let status = Status::of_ordinal(i64::from(i.int()?)).ok_or("an event of unknown status from the test bridge")?;
                    let thrown = if i.bool()? { Some(i.throwable()?) } else { None };
                    let ms = i.long()?;
                    let suite = self.suite.clone();
                    let lines = tally.add(Event { suite, name, selector, status, thrown, ms, crashed: false }, self.verbose);
                    if !lines.is_empty() {
                        (self.out)(&lines);
                    }
                }
                op @ LOG_ERROR..=LOG_TRACE => {
                    let _run = i.int()?;
                    let _logger = i.int()?;
                    let (prefix, shown) = match op {
                        5 => ("[error] ", true),
                        6 => ("[warn] ", true),
                        7 => ("", true),
                        8 => ("[debug] ", self.verbose),
                        _ => ("", self.verbose),
                    };
                    let text = if op == LOG_TRACE { i.throwable()?.trace() } else { i.string()? };
                    if shown {
                        let lines: String = text.lines().map(|l| format!("{}{}\n", prefix, l)).collect();
                        (self.out)(&lines);
                    }
                }
                // A framework's messages between its runners, which a run without workers has
                // none to pass between.
                _ => {}
            }
        }
    }

    /// The frameworks the export names, each by its class (`detectFrameworks`, one name a
    /// framework): what the bridge found of each, `None` where the link holds no such framework.
    pub fn detect(&mut self, classes: &[String]) -> Result<Vec<Option<FrameworkInfo>>, String> {
        let mut payload = Out::new();
        payload.int(classes.len() as i32);
        for c in classes {
            payload.strings(std::slice::from_ref(c));
        }
        let reply = match self.call(DETECT_FRAMEWORKS, &payload, &mut Tally::default())? {
            Ok(r) => r,
            Err(thrown) => return Err(format!("the test bridge did not look the frameworks up: {}", thrown.text)),
        };
        let mut i = In { bytes: &reply, at: 0 };
        i.list(|i| {
            i.option(|i| {
                let implementation = i.string()?;
                let _name = i.string()?;
                Ok(FrameworkInfo { implementation, fingerprints: i.list(In::fingerprint)? })
            })
        })
    }

    /// A framework's suites run with its arguments, as `RunnerAdapter` and `TaskAdapter` run them:
    /// a controller runner, its tasks, each executed through the continuation and the tasks it
    /// returns after it, then the runner's `done`, whose summary is written. A suite whose task
    /// threw, or the framework's runner, is an event of its own (`Event::crashed`); a channel that
    /// fails is the error.
    pub fn run_framework(&mut self, run: i32, implementation: &str, args: &[String], suites: &[TaskDef], ansi: bool, tally: &mut Tally) -> Result<(), String> {
        let crash_all = |me: &mut JsRun, tally: &mut Tally, thrown: Thrown| {
            for d in suites {
                let lines = tally.add(Event::crashed(&d.name, Some(thrown.clone())), me.verbose);
                (me.out)(&lines);
            }
        };
        let mut args_out = Out::new();
        args_out.int(run).string(implementation).strings(args).strings(&[]);
        if let Err(thrown) = self.call(CREATE_CONTROLLER_RUNNER, &args_out, tally)? {
            crash_all(self, tally, thrown);
            return Ok(());
        }
        let mut defs = Out::new();
        defs.int(run).int(suites.len() as i32);
        for d in suites {
            defs.task_def(d);
        }
        let tasks = match self.call(TASKS, &defs, tally)? {
            Ok(reply) => In { bytes: &reply, at: 0 }.list(In::task_info)?,
            Err(thrown) => {
                crash_all(self, tally, thrown);
                return self.done(run, implementation, tally);
            }
        };
        for task in tasks {
            self.execute(run, task, ansi, tally)?;
        }
        self.done(run, implementation, tally)
    }

    fn execute(&mut self, run: i32, task: TaskInfo, ansi: bool, tally: &mut Tally) -> Result<(), String> {
        let mut request = Out::new();
        request.int(run).task_info(&task).int(1).bool(ansi);
        self.suite = task.name.clone();
        match self.call(EXECUTE, &request, tally)? {
            Ok(reply) => {
                for next in (In { bytes: &reply, at: 0 }).list(In::task_info)? {
                    self.execute(run, next, ansi, tally)?;
                }
            }
            Err(thrown) => {
                let lines = tally.add(Event::crashed(&task.name, Some(thrown)), self.verbose);
                (self.out)(&lines);
            }
        }
        Ok(())
    }

    fn done(&mut self, run: i32, implementation: &str, tally: &mut Tally) -> Result<(), String> {
        let mut payload = Out::new();
        payload.int(run);
        match self.call(DONE, &payload, tally)? {
            Ok(reply) => {
                let summary = In { bytes: &reply, at: 0 }.string()?;
                if !summary.is_empty() {
                    (self.out)(&format!("{}\n", summary));
                }
            }
            Err(thrown) => {
                let lines = tally.add(Event::crashed(implementation, Some(thrown)), self.verbose);
                (self.out)(&lines);
            }
        }
        Ok(())
    }

    /// The run's end: the channel closed, which ends node's program, and node waited for, its
    /// output read to its end. A status other than 0 is the error.
    pub fn finish(mut self) -> Result<(), String> {
        let _ = self.writer.shutdown(std::net::Shutdown::Both);
        let status = self.wait_exit(EXIT_BOUND);
        for r in self.readers.drain(..) {
            let _ = r.join();
        }
        match status {
            Some(s) if s.success() => Ok(()),
            Some(s) => Err(format!("node ended with {} once the run had completed", s)),
            None => Err("node's end could not be waited for".to_string()),
        }
    }

}
