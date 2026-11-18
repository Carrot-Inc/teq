#!/usr/bin/env -S scala-cli shebang
!#
    def f: Unit =
    println("one")
    println("two")
@main def main(): Unit = f
// The lines of tests/scripts/indented-first-line-header-block.sc as a plain source after the same header: the
// first line after the header, four spaces in, is the outermost region's width, so the line after `=` at that
// width is the right side and the next one a statement at the top level. scalac: "Illegal start of toplevel
// definition" at the second println. (The text comes after the program, so that the first token stands right
// after the header.)
// expect: 5:5: error: expected a definition, found identifier
