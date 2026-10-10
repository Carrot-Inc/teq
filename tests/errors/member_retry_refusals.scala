// expect: 27:26: error: type mismatch: found String, required Int
// expect: 22:23: error: value toLowerCase is not a member of Int
// expect: 23:22: error: type mismatch: found String
// expect: 26:29: error: type mismatch: found Int, required String
// absent: 24:23:
// The refusal rows of a member's retry on its qualifier, as scalac's: an error inside an argument
// makes it an error argument whose member's errors stand (`hasInnerErrors`: a lambda's body that
// selects what its parameter lacks, a sequence built against the splice's element type); a
// definition the member's application completes keeps its own error, which fails no attempt, and
// the retry takes the call with no error of the member's (`Namer.Completer.complete`); a named
// argument's value is cached adapted to the member's formal (`typedNamedArg`), so the extension's
// `Int` does not take the `Long` it became.
class C1 { def f(x: Int => Int): String = "member" }
extension (c: C1) def f(x: String => String): String = "extension"
class S1 { def f(xs: Int*): String = "member" }
extension (s: S1) def f(xs: String*): String = "extension"
class D1 { def f(x: Int, y: String): Int = 0 }
extension (d: D1) def f(x: Int, y: Int): Int = 42
class P1 { def f(x: Long, y: String): String = "member" }
extension (b: P1) def f(x: Int, y: Int): String = "extension " + x
@main def run(): Unit =
  println(C1().f(x => x.toLowerCase))
  println(S1().f(Seq("s")*))
  println(D1().f(bad, 3))
  val i: Int = 4
  println(P1().f(x = i, y = 1))
def bad = { val x: Int = "bad"; 1 }
