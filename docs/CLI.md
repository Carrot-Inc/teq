# The command line

`teq` is one binary with no runtime. In a build exported by sbt-teq (`docs/TOOLING.md`) its verbs take the
projects of `teq.lock`, found at or above the working directory, and the launchers `./teq` and `teq.cmd`
beside the lock run the pinned compiler with nothing installed:

```
teq test [project] [pattern...]   # compile, then run the project's suites
teq run api [-- args]             # compile, then run the project's main class on a JVM
teq compile [project...]          # type the projects' closures and write their class files
teq dev frontend                  # a Scala.js project's dev loop; stage, watch, build and stop likewise
```

The compiler on files is `teq compiler`, and the interpreter `teq interp`:

```
teq compiler build src/ -o out/main.js     # compile files and directories
teq compiler build src/ --split out/       # one ES module per package instead (docs/TARGETS.md)
      --module-per-file pkg,...             # and one per file for these packages and the ones below them
      --hot                       # modules that a page can re-execute once it has booted
teq compiler watch src/ --split out/       # the same, then rebuild on commands from stdin
teq compiler watch src/ -o out/main.js     # the one file resident likewise, answering whether it is an ES module
teq compiler build src/ --target jvm -o out/main.jar   # JVM class files linked against scala-library's
                                  # jar, which runs beside them (java -cp out/main.jar:scala-library.jar
                                  # TeqMain); without .jar, -o names a directory of .class files
node out/main.js                  # a build runs under node (script or ES module), a jar under java
teq interp src/ [-- args]         # run in the compiler's own interpreter, without an output file
teq compiler check src/                    # type check only
      --classpath a.jar:b.jar     # read the classes the program names from the jars' TASTy files
      --std=scala-library         # on JavaScript, scala-library as the std from its jar (default: lean,
                                  # teq's own); the JVM's std is always scala-library's jar, named
                                  # with --classpath or taken from the coursier cache: 3.8.4 where it
                                  # is there, else the newest release teq reads (3.8 and older)
      --no-cache                  # read the jars without the jar cache
      --no-outline                # write each inline expansion at its site (docs/RUNTIME.md, "Bundle size")
      --exclude path              # leave out files whose path starts or ends with the text
      --strict-equality           # scalac's -language:strictEquality
      --kind-projector            # scalac's -Xkind-projector: `Either[String, *]` as a type lambda
      --max-inlines n             # scalac's -Xmax-inlines (32 by default)
      --dialect no-overloading,.. # turn costly constructs off, or `strict` for all (docs/SPEED.md)
      --werror                    # warnings fail the build
      --wunused imports,privates  # scalac's -Wunused, with its kinds
      --deprecation, --feature    # scalac's -deprecation and -feature: each warning in full; without them
                                  # a summary, which --werror counts too
      --wtostring-interpolated    # scalac's -Wtostring-interpolated
      --wconf 'id=E198:s,any:e'   # scalac's -Wconf: filters and actions, the rightmost rule winning
      --source 3.0-migration      # scalac's -source: the source version of a file whose source version
                                  # import names none (docs/COMPATIBILITY.md, "The source version")
      --source-at <dir> <version> # the source version of the files under a directory: another module's,
                                  # in a build of a closure (the project verbs pass it)
      --language implicitConversions  # scalac's -language
      --threads n                 # workers typing the bodies; by default one under 512 KiB of source,
                                  # else the cores, at most 8 (docs/TARGETS.md, "The typer's workers")
      --time                      # print per-phase timings
      --profile[=out.json]        # where the time of the type phase went, per construct and site
teq tasty lib.jar [name-filter]   # print the signatures a jar's TASTy files hold
teq tasty --body lib.jar a.b.C.m  # a definition with its body in Scala-like syntax
      --stats lib.jar...          # how many signatures map onto teq's types, and what blocks the rest
      --load lib.jar...           # enter every class and decode every signature through the typer
```

`teq interp` runs a program in the compiler's interpreter: the files and directories named, one main among them
(`@main def` or an object's `main`), the arguments after `--` its own. A file it runs can be a script:

- A first line `#!/usr/bin/env -S teq interp` is skipped, as scalac skips it (with scalac's `!#` closing a longer
  header), so that the same file runs under scala-cli. A script runs as `teq interp x.scala`, not by its path:
  `env` finds no `teq` on a plain `PATH`, and a project's launcher there would run that project's release.
- A Scala CLI script, `x.sc`, runs as Scala CLI 1.17.1 runs it: its statements, in their order, are the body of
  an object named after the file (`x_sc`, over the class `x$_`, as Scala CLI's `ClassCodeWrapper` writes them),
  `args` is the arguments after `--`, and an error points at the script's own line. A directory named or included
  brings its `.sc` files with its `.scala` ones, under `teq compiler check` too, each wrapped alike; the first script
  named is the one that runs.
- A directive is a line among the comments that open a file starting `//> using` exactly, its words apart by blanks
  (a tab as a space), a backtick-quoted word its identifier (`` `dep` `` is `dep`), a double-quoted one decoded
  (`\n`, `\t`, `\r`, `\\`, `\"`, `\u` and four hexadecimal digits, any other escaped character itself), as Scala
  CLI 1.17.1's lexer reads them. `//>using` is none and ends the directives there, `//>  using` ends them after
  its line with Scala CLI's warning. A block comment whose closing line holds more than blanks, another comment
  included, is code, and a directive after code is ignored with Scala CLI's warning. A key that is no identifier
  (a string, an empty or unterminated backtick-quoted word) or no key, an unterminated string and an invalid
  unicode escape stop the run (status 1) with Scala CLI's error.
- `//> using file <path>` and `//> using files <path> <path>...` among the comments that open a file bring other
  sources in: each path relative to the file that names it, quoted or not; a directory, the `.scala` (and `.sc`)
  files below it, hidden ones left out; each file once. A path that names nothing stops the run (status 2) with the
  file and the line. An included file's own directives are followed, which scala-cli (1.17.1) refuses. The `-source`
  among the scalac options of `//> using options` (`option`, `scalacOptions`), `-source:<v>` or `-source <v>`, is the
  program's source version unless the command line names one, as scala-cli puts the command line's options after
  the directives'; the first file's in the program's order where several name one. Every other `using` key but the
  dependencies' stays a comment, as it is to scalac.
- `//> using dep <organization>::<name>:<version>` (`::` the Scala suffix, `:::` the full Scala version,
  `,classifier=<c>` a classifier, quoted or not: a comma before no white space is the value's, as Scala CLI's
  lexer keeps it; another parameter, which the lock cannot check, stops the run), and every other key whose jars
  Scala CLI's run puts on the classpath (`deps`, `dependency`, `dependencies`, the deprecated `lib` and `libs` with
  its warning, `compileOnly.dep` and its like; `test.dep` and `scalafix.dep` have no part in a run), resolves through the lock of
  the build the script is in, since teq resolves no
  dependency itself: `teq.lock`, or `target/teq/teq.lock`, at or above the script's directory (`sbt teqExportAll`
  writes it). The script runs on the classpath of the build's JVM configuration that owns the sources it includes
  (its `//> using file` lines), or of `--module <project>` (`<project>/<configuration>`), in the export's order,
  its jars fetched as the build's verbs fetch them. The configurations whose products that classpath holds (its
  product closure) run with it from their sources, as the build's verbs type them, each at its
  configuration's source version (the `-source` the lock records), less the files `--exclude` leaves out, which
  are neither typed nor read for their directives; a closure its build keeps
  apart (`transitive: false`, a configuration that does not see another) or whose sources wait for a generator
  the build tool runs stops the run (status 2). Each coordinate must be on that classpath at the version asked,
  or at the one the lock records the build's resolution chose in its place (`evictions`, docs/TOOLING.md). A
  coordinate the lock does not hold, a version it does not, sources two configurations own alike, none owned, and
  a script outside every build stop the run (status 2) with a sentence naming what to do. With `--classpath` the jars
  are those it names. Inside a project (a configuration owning the sources, or `--module`) the lock still validates
  every coordinate, its version and evictions as without it, and sources two configurations own still stop the run.
  Outside one (no build, or one owning none of the sources and no `--module`) each coordinate must name one of the
  given jars by its artifact and version (a `<name>-<version>.jar`, or one under `<name>/<version>/`), and one that
  names none stops the run alike, `--classpath ""` included. The artifact is the run's platform's: where the given
  jars hold Scala.js artifacts the run is Scala.js's, and `org::name:1.0` names `name_sjs1_3`, as Scala CLI names
  it on Scala.js; otherwise it names the JVM's `name_3`. A lock's coordinates are the JVM's whatever
  `//> using platform` says, since the interpreter runs a build's JVM artifacts.
- `teq compiler` and the language server read no directive and see a script alone (a `.sc` one wrapped as above):
  a check names what the script includes, `teq compiler check tools/script scripts/x.scala`.
- In a project the launcher runs its scripts (`./teq interp scripts/x.scala`), so the release `teq.lock` pins
  interprets them as it compiles the project, the same on every machine; `TEQ` names another binary.

The interpreter gives a script the JDK it needs: processes (`ProcessBuilder`, `Process`, `ProcessHandle`), their
streams and the program's stdin, `java.nio.file.Files` and a file's byte channel, TCP sockets, zip and gzip
archives, `MessageDigest`, `System.getProperty`'s platform values and the clock (`Instant`, `ZonedDateTime`,
`DateTimeFormatter`), threads (`java.lang.Thread`, `synchronized`, `Object.wait` and `notify`, and
`java.util.concurrent`'s executors, locks, queues, latches, futures and atomics) and shutdown hooks, each as the
JDK behaves; zio's runtime runs on them. The threads take turns on one OS thread (docs/RUNTIME.md, "The
interpreter"). The program ends when its main has returned and no thread but daemons is alive; then, at
`System.exit`, at an exception main does not catch, or at SIGINT, SIGTERM or SIGHUP (status 128 plus the signal's
number), its shutdown hooks run one after another on the main thread while the other threads go on, and then every
thread stops. A second signal ends the program at once.

An argument `@file` stands for the file's lines, one argument per line, up to the first `--`
(what follows a `--` is the program's, as written): `teq @build.args`. teq's own tools start teq
this way, since Windows refuses a command line over 32,767 characters, which a class path of a few
hundred jars runs past.

Running the output needs node (any recent version; `Long` uses BigInt) for JavaScript and
`java` 21 or later for the JVM target. The compiler itself needs nothing.
