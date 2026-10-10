// teq: --werror
// A member's arguments are typed in their order, a function literal among them, as dotty's
// `matchArgs` types them: the lambda is cached before the later argument fails, and the retry
// adapts it as typed, so the transparent macro in its body ran once, in the member's attempt, whose
// warning went with it (scalac accepts this under `-Werror` and prints the same).
class B { def f(x: => (() => Int), y: String): Int = 0 }
extension (b: B) def f(x: => (() => Int), y: Int): Int = x() + x() + y
class C { def f(x: () => Int, y: String): Int = 0 }
extension (c: C) def f(x: () => Int, y: Int): Int = x() + y
@main def run(): Unit =
  println(B().f(() => M.next, 10))
  println(C().f(() => M.next, 20))
