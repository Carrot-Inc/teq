// expect: Main.scala:21:13: warning: A pure expression does nothing in statement position
// expect: Main.scala:17:17: warning: macro warning
// expect: 2 warnings found, errors under --werror
// absent: Main.scala:18:
// absent: Main.scala:19:
// teq: --werror
// The diagnostics of a member's retry on its qualifier, as scalac's: a transparent macro written as
// the argument is expanded at each adaptation and warns again where the extension's application
// expands it, once; one in braces is expanded in the member's typing, which the retry reuses, and
// its warning goes with the member's failed attempt (`TyperState`), as does the typer's own warning
// in an argument; a definition the member's application completes keeps its warning
// (`Namer.Completer.complete`), which fails the build under fatal warnings. scalac 3.8.4 gives the
// same warnings without `-Werror` and fails with it.
class B { def f(x: Int, y: String): Int = 0 }
extension (b: B) def f(x: Int, y: Int): Int = x + y
@main def run(): Unit =
  println(B().f(W.next, 3))
  println(B().f({ W.next }, 3))
  println(B().f({ 1; 2 }, 3))
  println(B().f(bad, 3))
def bad = { 1; 2 }
