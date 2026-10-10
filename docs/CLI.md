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
teq compiler build src/ --target jvm -o out/main.jar   # JVM class files linked against scala-library's
                                  # jar, which runs beside them (java -cp out/main.jar:scala-library.jar
                                  # TeqMain); without .jar, -o names a directory of .class files
node out/main.js                  # a build runs under node (script or ES module), a jar under java
teq interp src/ [-- args]         # run in the compiler's own interpreter, without an output file
teq compiler check src/                    # type check only
      --classpath a.jar:b.jar     # read the classes the program names from the jars' TASTy files
      --std=scala-library         # on JavaScript, scala-library as the std from its jar (default: lean,
                                  # teq's own); the JVM's std is always scala-library's jar, named
                                  # with --classpath or taken from the coursier cache
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
- `//> using file <path>` and `//> using files <path> <path>...` among the comments that open a file bring other
  sources in: each path relative to the file that names it, quoted or not; a directory, the `.scala` files below
  it, hidden ones left out; each file once. A path that names nothing stops the run (status 2) with the file and
  the line. An included file's own directives are followed, which scala-cli (1.17.1) refuses. Every other `using`
  key stays a comment, as it is to scalac.
- `teq compiler` and the language server read no directive and see a script alone: a check names what the
  script includes, `teq compiler check tools/script scripts/x.scala`.
- In a project the launcher runs its scripts (`./teq interp scripts/x.scala`), so the release `teq.lock` pins
  interprets them as it compiles the project, the same on every machine; `TEQ` names another binary.

The interpreter gives a script the JDK it needs: processes (`ProcessBuilder`, `Process`, `ProcessHandle`), their
streams and the program's stdin, `java.nio.file.Files`, TCP sockets, zip and gzip archives, `MessageDigest`,
`Thread.sleep`, shutdown hooks, `System.getProperty`'s platform values and the clock (`Instant`,
`ZonedDateTime`, `DateTimeFormatter.ofPattern`), each as the JDK behaves; it has no threads, and runs a shutdown
hook on its one thread when the program ends: at the end of its main, at `System.exit`, at an uncaught exception,
or at SIGINT, SIGTERM or SIGHUP (status 128 plus the signal's number).

An argument `@file` stands for the file's lines, one argument per line, up to the first `--`
(what follows a `--` is the program's, as written): `teq @build.args`. teq's own tools start teq
this way, since Windows refuses a command line over 32,767 characters, which a class path of a few
hundred jars runs past.

Running the output needs node (any recent version; `Long` uses BigInt) for JavaScript and
`java` 21 or later for the JVM target. The compiler itself needs nothing.
