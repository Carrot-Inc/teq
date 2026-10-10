#!/bin/sh
# lib's generator (build.sbt's TeqCommand): Generated.scala into the directory the command is given last.
mkdir -p "$1/lib" && printf 'package lib\n\nobject Generated { val word: String = "hello" }\n' > "$1/lib/Generated.scala"
