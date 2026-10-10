// expect: Main.scala:9:39: warning: same expansion
// expect: 1 warning found, errors under --werror
// teq: --werror
// The extension a member's retry applies expands a dependent-result transparent argument again and
// warns again, as scalac's adaptation does: the member's attempt's warning goes with it, the
// extension's stays, and fails the build under fatal warnings (scalac 3.8.4 under `-Werror` too).
class B { def f(x: Int, y: String): Int = 0 }
extension (b: B) def f(x: Int, y: Int): Int = x + y
@main def run(): Unit = println(B().f(M.same(10), 3))
