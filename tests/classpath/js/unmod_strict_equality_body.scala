// jars: fixtures
// teq: --strict-equality
//> using options -language:strictEquality
// A library body comparing a case class that has no `CanEqual`, called from a program compiled
// under strict equality: the library was checked apart, as scalac compiled it; the expectation is
// scalac 3.8.4's.
import fix.unmod.*

@main def main(): Unit =
  println(UmEq.same(UmPt(1, 2), UmPt(1, 2)))
  println(UmEq.same(UmPt(1, 2), UmPt(2, 1)))
