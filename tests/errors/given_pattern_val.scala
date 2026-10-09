// expect: 9:11: error: given patterns are not allowed in a val definition,
// expect: 10:9: error: given patterns are not allowed in a val definition,
// A `given T` pattern binds a given in a case or a `for`; in a val definition scalac rejects it
// (scala3's tests/neg/i11897.scala).
case class A(i: Int)
case class D(a: A)

def test =
  val (x, given A) = (1, A(23))
  val D(given A) = D(A(1))
  x
