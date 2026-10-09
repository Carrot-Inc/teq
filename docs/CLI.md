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
      --threads n                 # workers typing the bodies; by default one under 512 KiB of source,
                                  # else the cores, at most 8 (docs/TARGETS.md, "The typer's workers")
      --time                      # print per-phase timings
      --profile[=out.json]        # where the time of the type phase went, per construct and site
teq tasty lib.jar [name-filter]   # print the signatures a jar's TASTy files hold
teq tasty --body lib.jar a.b.C.m  # a definition with its body in Scala-like syntax
      --stats lib.jar...          # how many signatures map onto teq's types, and what blocks the rest
      --load lib.jar...           # enter every class and decode every signature through the typer
```

An argument `@file` stands for the file's lines, one argument per line, up to the first `--`
(what follows a `--` is the program's, as written): `teq @build.args`. teq's own tools start teq
this way, since Windows refuses a command line over 32,767 characters, which a class path of a few
hundred jars runs past.

Running the output needs node (any recent version; `Long` uses BigInt) for JavaScript and
`java` 21 or later for the JVM target. The compiler itself needs nothing.
