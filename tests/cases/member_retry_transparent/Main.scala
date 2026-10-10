// A member's retry on its qualifier adapts each argument its application cached as typed
// (dotty's `FunProto.typedArg`): a transparent call that is the argument itself, through
// parentheses, is cached unexpanded and expanded at each adaptation, the member's and the
// extension's (`Typer.adapt`), so its macro runs twice; one in braces or inside another tree is
// expanded in its typing, once; a plain one waits for the later phase and runs once; a clean
// argument typed after the member's error is not cached and is typed again by the retry's test.
class B { def f(x: Int, y: String): Int = 0 }
extension (b: B) def f(x: Int, y: Int): Int = x * 100 + y
class C { def f(x: String, y: Int): Int = 0 }
extension (c: C) def f(x: Int, y: Int): Int = y
@main def run(): Unit =
  println(B().f(T.next, 3))
  println(B().f((T.next), 3))
  println(B().f({ T.next }, 3))
  println(B().f(T.next + 0, 3))
  println(C().f(1, { val n = T.next; n }))
  println(T.next)
  println(B().f(P.next, 3))
  println(P.next)
