// teq: --werror
// A wildcard after a split product deeper than the reachability walk follows is left alone:
// the walk's answer is unknown there, so neither the null exemption nor the warning applies.
case class P[A](p: A)
type Deep = P[P[P[P[P[P[P[P[P[P[P[P[P[P[P[P[P[Option[Int]]]]]]]]]]]]]]]]]]

def f(p: Deep): Int = p match
  case P(P(P(P(P(P(P(P(P(P(P(P(P(P(P(P(P(Some(_)))))))))))))))))) => 1
  case P(P(P(P(P(P(P(P(P(P(P(P(P(P(P(P(P(None))))))))))))))))) => 2
  case _ => 3

@main def run(): Unit =
  val some: Deep = P(P(P(P(P(P(P(P(P(P(P(P(P(P(P(P(P(Some(1))))))))))))))))))
  val none: Deep = P(P(P(P(P(P(P(P(P(P(P(P(P(P(P(P(P(None)))))))))))))))))
  println(f(some) + f(none) + f(null))
