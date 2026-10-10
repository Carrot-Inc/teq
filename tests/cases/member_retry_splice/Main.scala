// A spliced sequence is the wildcard-star argument's typing, which types and adapts it against a
// sequence or an array of the elements (dotty's `typedWildcardStarArgExpr`): a transparent call
// there expands as the argument is typed, before the calls after it, and a member's cache keeps it
// expanded, so a retry's reuse expands nothing again; typed after an earlier argument's mismatch,
// or not fitting the member's elements itself, it is not cached, and the retry's test types it
// again. A string is spliced as its characters through the conversion to a sequence.
class B { def f(ys: Int*): Int = ys.head }
class C { def f(x: String, ys: Int*): Int = 0 }
extension (c: C) def f(x: Int, ys: Int*): Int = ys.sum
class E { def f(ys: String*): Int = 0 }
extension (e: E) def f(ys: Int*): Int = ys.head
def chars(cs: Char*): String = cs.reverse.mkString
@main def run(): Unit =
  println(B().f(M.arr*))
  println(M.next)
  println(C().f(1, M.arr*))
  println(M.next)
  println(E().f(M.arr*))
  println(M.next)
  println(chars("ab"*))
