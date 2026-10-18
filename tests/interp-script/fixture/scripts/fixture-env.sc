#!/usr/bin/env -S scala-cli shebang -q
//> using scala 3.8.4
//> using file ../deps.scala
//> using file ../tools/src

// The tool's launcher: its code is the build's `tools` project's, its dependencies the lock's.
envfix.Main.main(args)
