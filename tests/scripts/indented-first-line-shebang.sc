#!/usr/bin/env -S scala-cli shebang
    def f: Unit =
    println("one")
    println("two")
f
// A script whose first line after a one-line `#!` header is indented: Scala CLI writes the script's statements
// into its wrapper class's braces, after the class's own lines at the start of a line, so the lines four spaces in
// are further in than the braces' statements and `f`'s body after `=` is an indented block of both lines. (The
// text comes after the program, so that the first token stands right after the header.) Scala CLI prints one,
// then two.
