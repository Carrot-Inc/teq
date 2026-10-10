// A transparent call whose result type depends on its parameter is cached unexpanded as any other
// and expanded at each adaptation (dotty's `typedArg` and `adapt` ask nothing of the result type):
// the member's attempt and the extension's application expand `same` once each.
class B { def f(x: Int, y: String): Int = 0 }
extension (b: B) def f(x: Int, y: Int): Int = x + y
@main def run(): Unit =
  println(B().f(M.same(10), 3))
  println(M.next)
