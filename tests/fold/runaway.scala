// The compile-time evaluation of an inline argument that never ends (tests/fold.sh; the program is
// built, never run): the std's loop goes on in the interpreter's lowered code once it has turned a
// few times (src/interp/code.rs), and the budget of 50,000 steps stops it at a loop's head, the
// argument staying a computation in the output.
inline def twice(inline x: Int): Int = x + x
@main def run(): Unit = println(twice(Iterator.continually(1).sum))
