package outer
class Anchor
package inner:
  class C { def f(x: Int): Int = x }
  val n = new C `f` 1
  val m = new C f 1
  class Pair[A, B]
  type X = Int `Pair` String
  type Y = Int Pair String
  case class P2(a: Int, b: Int)
  def g(p: P2): Int = p match { case a `P2` b => a + b }
  def h(p: P2): Int = p match { case a P2 b => a + b }
