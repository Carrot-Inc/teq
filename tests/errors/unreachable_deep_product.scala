// expect: warning: unreachable case
// expect: 1 warning found, errors under --werror
// teq: --werror
// One pattern covers the whole product, however deep: the wildcard after it is reachable
// through null alone, which scalac reports as well.
case class P1(p: P2)
case class P2(p: P3)
case class P3(p: P4)
case class P4(p: P5)
case class P5(p: P6)
case class P6(x: Int)

def f(p: P1): Int = p match
  case P1(P2(P3(P4(P5(P6(_)))))) => 1
  case _ => 2
