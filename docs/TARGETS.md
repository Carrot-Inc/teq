# Targets

Teq compiles one typed program to three targets: JavaScript, JVM class files and its own
interpreter. This document says what each target's output is like and how it runs, how JavaScript and
Scala.js facades are reached, what the resident watch mode does, how the typer's workers are counted, what
`--time` reports and what the Windows binary follows. The commands are in `docs/CLI.md`, the libraries a
build reads in `docs/LIBRARIES.md`, the output's speed in `docs/RUNTIME.md`.

## JavaScript

`teq compiler build src/ -o out/main.js` writes one file meant to be read in the browser's inspector as it is: classes
for classes, methods for methods, `const` and `let` for locals, arrow functions for lambdas, `if` chains for
matches, indented, with the names of the source. What the program never reaches is left out, so hello world
is a few kilobytes. Running the output needs node (any recent version; `Long` uses BigInt).

- The output runs the entry point, a `@main` method, an object's `main` (inherited ones included) or a
  package's own `main(args: Array[String])`, and exports what `@jsExport` names; a build with neither is
  rejected with `the program has no entry point`. `--main` takes the name `java` runs it by (`app.Main`, a
  `@main def run`'s `app.run`, a package's `app.Main$package`) or its simple one.
  A program with an import or export is an ES module (`out/main.mjs` unless `-o` says otherwise).
- A class, object, def or enum value goes by its bare name (`Address`, `Color$Red`, `Outer$Inner`) where no
  other definition of the program has it and qualified with its package otherwise; every name derives from
  the definitions alone, so the same sources give the same bytes. An object is created on the first call of
  its accessor, `Registry$()`; an anonymous class that is a closure in all but name is an arrow function; a
  concatenation is rendered in Scala.js's order rather than the JVM's (`docs/COMPATIBILITY.md`).

### One module per package: `--split`

`teq compiler build src/ --split out/` writes a directory of ES modules for a bundler's development server, which then
serves, and the browser re-parses, only the modules an edit changed; a production build reads the directory
as an ordinary module graph. `main.mjs` is the entry, `rt.mjs` holds the runtime, `std.mjs` what the program
reaches of the standard library, and every package has a module named after it (`component.widget.mjs`).
Modules import exactly what they refer to from each other, cycles included. A module is rewritten only when
its text differs from the file on disk, and its text depends on its own definitions alone, so an edit
rewrites the module of its file. Hello world is four files of 3.9 KB.

### The development loop: `--module-per-file` and `--hot`

`--module-per-file component,page` makes every source file of the named packages, and of the packages below
them, a module of its own (`component.auth.LoginForm.mjs`), so that a server re-executing an edited module
with its importers runs the edited file and the few modules above it rather than a whole package. `--hot`
writes what such a swap needs in the terms of `import.meta.hot`, so that any server driving the loop takes
the policy from the output: a per-file module and the per-file modules that import it run again; a
package's module, the standard library and the runtime hold shared code whose classes the page's values
belong to, never run again, and reach the swapped definitions through bindings the runtime sets after each
swap. What cannot be swapped loads the page again: an edit of shared code, a swap of more than a hundred
modules, a per-file class that shared code extends, tests for or compares with. A value the page holds
across a swap of the file that defines its class matches nothing of the new classes, so the types a page
holds belong in a package that is not per file. Without `--hot` the output is unchanged.

### `--release`, sizes and the size report

`--release` writes JavaScript for production: the members of Scala classes get short names from one table for
the whole program, the output has no indentation or line breaks and the runtime no comments. A name
JavaScript code can see keeps its own: the members of JS types, what the runtime calls on any value, what a
`js.Dynamic` selection or a string passed to the `js` package names, and the members of every class an
exported definition takes or returns; the output runs through esbuild or another minifier as it is.
`--size-report` prints where the bytes went: the total and the runtime, a line per module, per package and
per jar, the anonymous classes by the expression that makes them, and the largest definitions. Sizes are
held to budgets for the development output, the release output, and the latter through esbuild and gzip.

### JavaScript interop

```scala
@jsImport("react", "createElement") def createElement(tpe: Any, props: Any, children: Any*): Any
@jsImport("node:path", "*")         val path: Any      // "default" for the default export
@jsExport("render")
def render(title: String): Any = createElement("h1", js.obj("className" -> "title"), title)
```

- `@jsImport("module", "name")` goes on a top-level `def` or `val` without a body; arguments are passed
  positionally and a repeated parameter is spread; bodies, `using` clauses, default arguments and by-name
  parameters are rejected. `@jsExport("name")` exports a top-level `def` or `val`, `"default"` the default
  export. Module specifiers are copied as written.
- JS values are typed `Any` and the `js` package works on them: `js.obj("a" -> 1)`, `js.get`, `js.set`,
  `js.call(target, "method", args*)`, `js.construct`, `js.global("document")`, the unchecked `js.cast[T](x)`.
- Nothing is converted at the boundary: teq functions are JS functions, `String`, `Boolean`, `Int` and
  `Double` the JS primitives, `Long` a BigInt, `Unit` `undefined`, `Array` a JS array, and the vals of a
  class plain properties. Every class with a `foreach` is iterable from JS (`[...list]`, `for … of`).

### Scala.js interop

The facade syntax of Scala.js is understood as it stands, so that an application's facade files compile
unchanged under scalac with Scala.js and under teq. `scala.scalajs.js` and `org.scalajs.dom` are part of the
standard library in Scala.js's names (`js.Object`, `js.Array`, `js.Dictionary`, `js.Dynamic`, `JSON`,
`UndefOr`, timers, typed arrays, the DOM); with a scalajs-dom jar on the class path the DOM is the jar's.

```scala
@js.native @JSImport("mapbox-gl", "LngLat")
final class LngLat(lng: Double, lat: Double) extends js.Object

val el = document.getElementById("x").asInstanceOf[js.Dynamic]
el.style.userSelect = "none"
```

- Bindings. `@JSImport("module", "name")` (with `JSImport.Default` and `JSImport.Namespace`), `@JSGlobal`
  and `@JSGlobalScope` go on a `def`, `val`, `object` or `class` at any nesting together with `@js.native`;
  `@JSName` and `@JSBracketAccess` rename and index members; `@JSExportTopLevel` exports as `@jsExport` does.
- Native types (`@js.native`, native parents) have no runtime class: a `val` or parameterless `def` is a
  property read, a `var` assignment a write, a def with parameters a JS method call whose trailing omitted
  arguments are not passed, `new C(args)` a `new` of the binding, a type test an `instanceof`. A class that
  extends `js.Object` without `@js.native` describes a plain JS object with one property per val, reference
  equality and JavaScript's `toString`; `new T { val a = e }` for a JS trait is an object literal.
- `js.Dynamic`: on such a receiver every member is typed `js.Dynamic` and emitted verbatim (`e.n`, `e.n = v`,
  `e.n(args)`); `js.Dynamic.literal(a = 1)` is an object literal and `js.Dynamic.global` bare identifiers.
- `js.Array[A]` is Scala's `Array[A]`, both the JavaScript array, with Scala.js's extra members (`push`,
  `pop`, `splice`, `length = n`). `js.UndefOr[A]` is `A | Unit`, `Null` the type of `null`, and `Option(x)`
  is `None` for `null` and `undefined`. `js.Function`, `js.ThisFunction0` to `6`, `js.constructorOf[C]`,
  `js.Thenable` with `toFuture` and `LinkingInfo` (`productionMode` is `--release`) have Scala.js's shapes.
- Facades from jars (scalajs-dom, an npm package's wrapper) compile as the same facade in source, their
  annotations read from TASTy. `Reflect.lookupInstantiatableClass` and `lookupLoadableModuleClass` find
  every class that carries or inherits `@EnableReflectiveInstantiation`, in the sources and in the jars.

Not yet: `@JSExportTopLevel` in a jar is not exported; a wildcard or `given` import from a stable val
(`import org.scalajs.dom.window.*`) is not supported; where `js.Array`, reflective instantiation and the
`classOf` of collections differ from Scala.js is listed in `docs/COMPATIBILITY.md`.

## JVM class files

```bash
teq compiler build src/ --target jvm -o out/main.jar     # a jar; without .jar, -o names a directory of class files
java -cp out/main.jar:scala-library-3.8.4.jar TeqMain
```

`--target jvm` lowers the same typed program to class files that link against scala-library's bytecode: the
standard library is scala-library's jar, named with `--classpath` or taken from the coursier cache, never
compiled or written, and `--std=lean` is refused. Nothing of any jar is converted or emitted; a call of a
jar member goes to the jar's class file with the descriptor that file declares, and `java` runs the output
beside the jars, as scalac's output runs (`teq run` passes them, and `-Xss512m`). The jar's manifest names
the launcher `TeqMain`. Running needs `java` 21 or later. `--java-output-version N` is scalac's flag: class
files of Java release `N`, 17 by default, 17 to 24 accepted. 1,020 programs of the conformance suite print
on the JVM what scalac's output prints, and the 51k-line API of the benchmark application builds over cats
and sourcecode and prints scalac's output.

What of scalac's encoding it follows, so that code scalac compiled links with it: traits are interfaces with
default methods, an object is `Name$` with a `MODULE$` field, the top-level definitions of `f.scala` are
members of `f$package$`, operators are `$`-encoded, vals have accessors (a plain constructor parameter only
its class's own code reads is a private final field of none), defaults are `m$default$N`, lambdas
are `invokedynamic` call sites, generics are erased as scalac erases them with casts and boxing at the
boundary, a value class erases to its underlying type, arrays are JVM arrays. A case class gets `canEqual`
and `productElementName`, a companion `C$` with `apply`, `unapply`, `fromProduct` under `Mirror.Product` and
the default getters; objects get static forwarders and mirror classes, enums `values` and `valueOf`; a
`given ... with` is scalac's class `Owner$x` that its owner's def `x(params)` answers, or without parameters
the module class `Owner$x$`, one of a class or trait instance holding it in `$outer`. `throw`
is `athrow`, `try` an exception table entry, the exception classes are the JDK's, and line numbers are
written per expression, so that a chain of calls over several lines attributes each call to its own line.

Not on the JVM: the JavaScript interop (`@jsImport`, facades, the `js` package beyond `isNull` and `cast`)
is reported as unsupported when a program reaches it; `--split`, `--module-per-file` and `--hot` are JS only.

Not yet, against scalac's layout: `Signature` attributes, specialised `FunctionN` entry points for library
lambdas, `FunctionXXL` above 22 parameters (a lambda of that many is an interface of its own arity, so a
cast to such a function type, which erases to `FunctionXXL`, checks nothing), `writeReplace`, and
`Serializable` on objects and lambdas (a case class, a tuple and an enum have it); a given is not `final`; an
object nested in an object is no static field of the outer one; a private member is public in bytecode under
its own name where scalac widens it under an expanded one; a `try` as a constructor argument and
`getStackTrace` are missing.
Class files are 2.4 times the JavaScript text, and lazy vals are not thread-safe.

## The interpreter

`teq interp src/` runs the program in the compiler itself, over the typed program the two
backends read, without writing an output; `--target interp` is no option of `teq compiler`, the interpreter
being a verb of its own. It types the bodies of the standard library as the program reaches them, prints what `println`
writes, and exits 1 with `Exception in thread "main" ...` on an uncaught exception, as the JVM does; a
construct the interpreter lacks prints what it is and exits 3. The same engine runs macros at compile
time, a quoted macro read from a jar's TASTy arriving as ordinary typed code, and evaluates `inline`
arguments: an argument made of literals and calls of the standard library (`"ab".length + 1`) becomes a
constant under a small budget, where an effect, an exception or an expensive computation leaves it as it
is. Every program of the conformance suite and 435 programs of the Scala 3 compiler's suite print under it
what scalac's output prints.

Where it differs: numbers format as on the JVM, so `println(1.0)` prints `1.0` where the JavaScript target
prints `1`, and `1e21` prints `1.0E21`; and a program runs in its own process, as on the JVM, which is what
a generator of sources needs (`docs/TOOLING.md`): it reads and writes the host's files through
`java.nio.file` (`Files`' readers and writers, `createDirectories`, `deleteIfExists`, `list` and `walk` as
streams its caller closes, `Path`'s `relativize`, `normalize`, `startsWith` and `toAbsolutePath`), with
the JDK's exceptions for a path that is not there, a directory and bytes that are not UTF-8; it sees its
working directory (`user.dir`), its environment and the system properties that follow from the process;
and `sys.exit(n)` ends `teq interp` with the status `n`. A macro sees the build's environment and files
as under scalac and writes none: its expansions run in parallel and may be retried, so an effect on disk
would have no order. Everything else follows the JavaScript runtime, so where that layer
is a Scala.js library's (`java.text` from scala-java-locales) the output is Scala.js's. The core-only
benchmark runs faster end to end than under node or java, since nothing is emitted and no VM starts; a
tight loop runs some 12 million operations a second, 35 times slower than node's whole process.

## Watch mode and the resident compiler

`teq compiler watch src/ --split out/`, with the options of `build`, `--module-per-file` and `--hot` included, builds
once, then stays resident with the parsed files and the typed program in memory and rebuilds on command. The
client, a bundler's plugin or the sbt plugin (`docs/TOOLING.md`), owns the file watching and tells teq what
changed. Commands come on stdin, one per line: `build` followed by the changed paths, one per line, and an
empty line; `build` alone checks the modification time of every file; `quit` or the end of stdin ends the
session, as does the end of the process that started it. Every build answers with one JSON line on stdout:

```json
{"ok":true,"changed":["component.auth.mjs"],"modules":187,"incremental":true,"retyped":["src/a.scala"],
 "ms":{"read":..,"parse":..,"type":..,"reach":..,"emit":..,"write":..,"total":..},"warnings":[]}
```

or `{"ok":false,"errors":[{"file":..,"line":..,"col":..,"message":..,"source":..,"caret":..}]}`. A build
that fails leaves the output directory as it was; a full build carries `"incremental":false` and its reason
in `"fallback"`.

What it re-types: an edit that changed only bodies is compared with the old syntax tree at signature level
(the same definitions, kinds, names, modifiers, parameter lists, declared result types, parents and
members, recursively), and the file's bodies alone are typed again against the existing symbol table; a val
or def whose inferred type comes out the same is seen by no other file. An edit of an inline body expanded
in other files types those files again too, and the answer's `retyped` names them. Everything else, a
signature changed, an inferred type changed, a file added or removed, takes the full path in the same
process with the unchanged files' syntax trees kept, and a watch build gives modules byte-identical to a
fresh `teq compiler build --split`. A macro's own state lasts one build, as under scalac; an object that holds
nothing but a cache is kept across retypes when `--cacheable-state a.b.Cache` names it.
`teq compiler watch src/ --target jvm -o out/classes` keeps a JVM build resident for a build tool: the class files
whose bytes changed are rewritten, the rest left alone, and an answer that typed files carries the classes
each defines (`analysis`) for the tool's discovery.

Hot swapping in a page is the `--hot` output above: the server re-executes the edited per-file module and
its importers, and the page reloads where a swap cannot hold. The figures, for a production application of
187 packages: the full build takes about 500 ms (type check 430 ms); a body edit of one file is typed in
0.8 ms and the build answers in 20–27 ms. In the vite loop the time from saving the edit to the page reload
is 35–45 ms (the first edit of a session about 250 ms), a type error is on the overlay after 60 ms, and a
signature change costs the full build, 530–640 ms.

## The typer's workers

A build types its bodies on several workers when the program is large enough, and its output is one
worker's whatever their count. By default a program under 512 KiB of source types on one worker, where the
fork's fixed cost is not paid back, and so does one whose largest top-level definition holds more than half
of its bytes; above that the count is the least of the machine's cores, eight, and one worker per GiB of
memory above the first. `--threads n`, or `TEQ_THREADS=n`, types with `n` workers past every bound and
`--threads 1` on one without the fork; in sbt, `teqThreads := Some(n)`. A session types its full builds at
that count and its retypes on one worker.

A macro whose run changes state other runs share gives the parallel attempt to one worker, which types the
build again in scalac's order: the output is the same, the attempt's time is lost (11 to 22% on the
benchmark application), and the build says so in one line. `--cacheable-state` declares a module that holds
nothing but a cache, after which its changes no longer give the build away, `--macro-state per-worker` keeps
each run's change to its worker, and `--threads 1` skips the attempt. On the 64k-line frontend the type
phase takes 3.2 s at one worker and 1.06 s at eight on a 16-core machine, the 51k-line API 5.5 and 1.4 s;
the memory is 1.23 GB at one worker and 2.42 GB at eight.

## The `--time` report

`--time` prints one report on stderr when the command is done: sections of rows, each with its count, its
time in milliseconds and a note (`classpath` for the jars, `Java` for the JDK, `library bodies` for the jar
methods typed; a section a command has nothing for is left out), and `phases` last: `lines`, then `read`,
`parse`, `type`, `reach`, `emit` and `write` in milliseconds, and `total`. `check` has `read`, `parse` and
`type`; a JVM build notes the classes and bytes written on its `write` row, a split build the modules
written; `teq interp` has `run` after `type`; a parallel build's `workers` row counts the workers
started. Watch mode has its phases in the answer's `ms` object instead; `--profile` is `docs/SPEED.md`.

## Platforms

A release carries one binary per platform, told apart by classifier: macOS on Apple silicon and on Intel
(`osx-aarch_64`, `osx-x86_64`; macOS 11 and later), Linux on x86-64 and on aarch64 (`linux-x86_64`,
`linux-aarch_64`; glibc 2.28 and later, so RHEL 8, Debian 10 and Ubuntu 18.10 onward; the aarch64 binary is
for ARMv8 servers and Docker on Apple silicon, not for Cortex-A53 cores), and Windows on x86-64
(`windows-x86_64`, below), each an asset of teq's GitHub release `v<version>`, `teq-<version>-<classifier>`
(`.exe` for Windows alone), with the release's `SHA256SUMS`, which the sbt plugin and the Zed extension check it against. Maven Central carries
the sbt plugin alone, `build.teq:sbt-teq`, on its own version line. A release carries the five, each run on its
platform before its first release. The releases before 0.1.7 are not served: the sbt plugin, the launchers and the
Zed extension refuse them.

## Windows

The Windows binary, `teq-<version>-windows-x86_64.exe`, is a console program for Windows 10 or later that
imports the system's DLLs and the UCRT alone. It is checked under wine on Linux, which has neither NTFS's
sharing rules nor a Windows console, and follows once it is qualified on Windows itself. What it follows:

- Path lists. `--classpath` and `teq run`'s `-cp` take `;` between entries, as javac and scalac do there.
- Paths. `\` and `/` both separate a path's components wherever teq knows a file by its name; a jar's entry
  names and a class's binary name keep `/`. Caches are `%LOCALAPPDATA%\teq` (`TEQ_CACHE_DIR` first) and
  coursier's `%LOCALAPPDATA%\Coursier\cache\v1`; temporary files go to `%TEMP%`. A macro's
  `java.nio.file.Path` follows the JDK's `WindowsPath`, so `Path.of("C:/work")` resolved with `sub/data.txt`
  is `C:\work\sub\data.txt`, as scalac's macro gives it there. The language server's URIs are
  `file:///C:/work/A.scala` and a share's `file://server/share/A.scala` (`\\localhost\c$\...` among them);
  paths past 260 characters work.
- Programs. `java` is `java.exe`, `JAVA_HOME`'s or the first on `PATH`; a program given by a bare name is
  found through `PATHEXT`, so `npm` is `npm.cmd`, run through `cmd.exe`, by teq and by sbt-teq's generators.
  The class path java gets is written in the system's code page, which java reads it in: a path with a
  character that page lacks is refused. The language server's automatic
  export is refused on Windows; `sbt teqExportAll` in the project is the step it names.
- The console. The watch session's stdin is peeked where it is a pipe and read line by line where it is a
  console; a pipe that cannot be peeked is relayed by a thread of its own. A build whose parallel attempt
  gives way runs again as a child process, the parent passing on its exit code and taking Ctrl-C and
  Ctrl-Break so that it outlives the child. A rename over a file another process holds open (a JVM reading
  a class file) is tried again ten times 100 ms apart, as sbt does; teq's own readers never block one.
