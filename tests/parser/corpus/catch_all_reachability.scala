// teq: --werror
// Catch-all cases on a reference scrutinee after cases that cover it only by splitting a
// product: `x @ _` and a guarded `_` take `null`, which the product patterns do not, so both
// are reachable, as scalac leaves them.
case class P(o: Option[Int])

def bound(p: P): Int = p match
  case P(Some(_)) => 1
  case P(None) => 2
  case x @ _ => 3

def guarded(p: P): Int = p match
  case P(Some(_)) => 1
  case P(None) => 2
  case _ if p == null => 3

@main def run(): Unit =
  println(bound(P(Some(1))) + bound(P(None)) + bound(null))
  println(guarded(P(Some(1))) + guarded(P(None)) + guarded(null))
