<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo/teq-lockup-dark.svg">
    <img alt="teq" src="assets/logo/teq-lockup-light.svg" width="320">
  </picture>
</p>

<p align="center">A native Scala 3 compiler, written in Rust</p>

<h1 align="center">Scala 3, compiled <br>in milliseconds.</h1>

Teq interprets or compiles Scala 3 to JVM or JavaScript.<br>Including unmodified libraries from the ecosystem.

**No JVM at compile time.** One native binary. Library macros run in the compiler's own interpreter.

**Your Scala libraries.** Use unmodified published libraries, with conformance checked against scalac.

**First-class JavaScript.** ES modules, granular recompilation with feedback in milliseconds, and lean compiled bundles.

## About the project

### Written with AI

The code, tests and documentation are written with AI agents. Humans direct the work and own the
design decisions.

<details><summary>Models and validation process</summary>

Implementation uses Claude Fable 5.1 and Claude Opus 5.5, with GPT 6 Astra reviewers. Each package
lands with test expectations generated against scalac, a full gate run, and its compile-time cost
measured and recorded. Read the code with that in mind.

</details>

## Status

### Pre-release, actively tested

Review the remaining gaps and intentional differences before adopting teq.

[Compatibility and limitations](docs/COMPATIBILITY.md)

Developed against a production Scala.js application. Conformance tests use scalac 3.8.4 as the
reference, including the Scala compiler's own suites.

Verified to work with:

- cats
- zio
- tapir
- monocle
- refined

and more

## Compilation speed

Wall-clock time of a full build on an Apple M4 Max.

| Input | | scalac, cold JVM | scalac, warm server | teq |
|---|---|---|---|---|
| Hello world | A two-line Scala `@main` program | 1.0 s | 0.3 s | 3.2 ms, 100–300× faster |
| Realistic Frontend | Synthetic · 64k lines · JavaScript target | 42 s | 36 s | 0.87 s, 41–49× faster |
| Realistic API | Synthetic · 51k lines · JVM target | 33 s | 29 s | 0.18 s, 160–180× faster |

The realistic benchmark is a 104k-line synthetic application generated to the shape of a production Scala.js code base.

[Benchmark methodology](docs/SPEED.md)

## Ways to use teq

### As the compiler of an sbt build

sbt's own `compile`, `test`, `run` and `packageBin` run over teq's output, with sbt's own reporting,
and nothing else in the build changes.

Add the plugin to an sbt 2 or sbt 1 build (the same line under both; the sbt 1 module comes with the plugin's first
release after 1.0.0):

```scala
// project/plugins.sbt
addSbtPlugin("build.teq" % "sbt-teq" % "1.0.0")
```

Enable scalac replacement:

```scala
// build.sbt
teqVersion := "0.1.8"
teqCompiler := true
```

The plugin comes from Maven Central; it fetches teq for your platform from teq's GitHub release, checked
against the release's checksums before it runs. Enable it per project, or use `TEQ_COMPILER=1` to switch
without editing the build.

[sbt integration guide](docs/TOOLING.md)

### As a native build tool

Compile, test and run without sbt running. One lockfile describes the build; the launcher fetches the
pinned compiler.

Add the [sbt plugin](#as-the-compiler-of-an-sbt-build) and set `teqBuildTool`:

```scala
// build.sbt
teqBuildTool := true
```

Then run `sbt teqExportAll` and commit the generated `teq.lock`, `teq` and `teq.cmd` files.

Drive your project without a JVM:

```bash
./teq compile api
./teq test
./teq run api -- web
```

Teq keeps a daemon compiler running so subsequent builds are even faster. The Vite plugin uses the
same lockfile for hot swapping in the browser.

[Native build tool guide](docs/TOOLING.md)

### In the editor

Use teq as you write Scala, with navigation, completion and diagnostics powered by the compiler.

- Definitions and references
- Hovers and completion
- Call hierarchy
- Diagnostics after each edit

**Open your project in Zed.** Install the teq extension and disable Zed's other Scala extension. Open
your project folder: the server reads `teq.lock`, or exports the sbt build on first use, and fetches
the pinned compiler. Without `teqBuildTool` that export lands in `target/teq/teq.lock`, adding nothing to
the repository, and runs again after an `sbt clean`.

**Or connect another editor.** Configure its language-server client to start this command in the
project folder, communicating over stdin and stdout.

```bash
teq lsp
```

[Editor setup guide](docs/TOOLING.md)

### As an interpreter

Run Scala directly, with no output file and no JVM, using the same interpreter that runs library macros.

```scala
// hello.scala
@main def hello(): Unit =
  println("hello from teq")
```

```bash
teq interp hello.scala
```

Download teq, rename the file to `teq` (`teq.exe` on Windows), make it executable and put it on your `PATH`. A
browser download on macOS carries the quarantine attribute, which Gatekeeper enforces on a binary that is not
notarized: `xattr -d com.apple.quarantine teq` clears it, and a download by `curl` never carries it.

- [macOS · Apple silicon](https://github.com/Carrot-Inc/teq/releases/download/v0.1.8/teq-0.1.8-osx-aarch_64)
- [macOS · Intel](https://github.com/Carrot-Inc/teq/releases/download/v0.1.8/teq-0.1.8-osx-x86_64)
- [Linux · x86-64](https://github.com/Carrot-Inc/teq/releases/download/v0.1.8/teq-0.1.8-linux-x86_64)
- [Linux · ARM64](https://github.com/Carrot-Inc/teq/releases/download/v0.1.8/teq-0.1.8-linux-aarch_64)
- [Windows · x86-64](https://github.com/Carrot-Inc/teq/releases/download/v0.1.8/teq-0.1.8-windows-x86_64.exe)

[Command-line reference](docs/CLI.md)

## Libraries

Published Scala 3 libraries come in from their jars as TASTy, signatures and bodies, through
`--classpath`; Java classes from their class files and the JDK's `ct.sym`. The bodies compile
on JavaScript against teq's own standard library, a lean one written for JavaScript and the
interpreter, whose protocol has grown where real libraries hit it: cats with kittens' derivation and
alleycats, zio and zio-json with Magnolia's derivation, tapir with the sttp client, monocle, refined,
chimney, izumi-reflect, scalajs-dom, scalajs-react, scala-java-time, sourcecode, atto and
case-insensitive run from their jars today, their macros included. `--std=scala-library` compiles the real scala-library from its jar instead, as a
fallback. A JVM build compiles no library: it links against the jars' bytecode, scala-library's
among them. `docs/LIBRARIES.md` has the details and the measurements.

## Documentation

| | |
|---|---|
| `docs/COMPATIBILITY.md` | Scala compatibility: what is not there yet, and where behaviour differs on purpose |
| `docs/CLI.md` | the command line: every command and flag |
| `docs/TARGETS.md` | the three targets and their output: JavaScript, JVM class files, the interpreter; watch mode, Windows |
| `docs/TOOLING.md` | the sbt plugin, the lockfile and the launchers, `teq`, the vite plugin, the language server and Zed |
| `docs/LIBRARIES.md` | libraries from their jars, the two standard libraries, Java classes |
| `docs/SPEED.md` | what makes teq fast, `--time` and `--profile`, the dialect flags |
| `docs/RUNTIME.md` | the output's runtime speed and size |

The notes for working on teq's code start at `docs/DEVELOPING.md`.

## Open source

### Build it yourself

Build from source and run the same suites used to check language conformance, output, runtime speed
and compilation budgets.

<details><summary>Build and test commands</summary>

```
cargo build --release
./tests/all.sh
./bench/pgo.sh
./bench/budget.sh
```

`all.sh` runs every suite. `pgo.sh` produces a profile-guided release build at
`target/pgo/use/release/teq`; `budget.sh` checks its compile times against `bench/budgets.txt`.

The suites compare output with scalac, use `scala-cli` to regenerate expectations, and run the Scala 3
compiler tests from `$SCALA3`.

</details>

## License

Apache License 2.0, see `LICENSE` and `NOTICE`. The tests adapted from the Scala 3 compiler's
suite name their original file in their header comment.
