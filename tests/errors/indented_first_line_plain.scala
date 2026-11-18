    def f: Unit =
    println("one")
    println("two")
@main def main(): Unit = f
// The lines of tests/scripts/indented-first-line.sc as a plain source: the first line, four spaces in, is
// the outermost region's width, so the line after `=` at that width is the right side and the next one a
// statement at the top level. scalac: "Illegal start of toplevel definition" at the second println. (The text
// comes after the program, so that the first token stands on the first line of the program.)
// expect: 3:5: error: expected a definition, found identifier
