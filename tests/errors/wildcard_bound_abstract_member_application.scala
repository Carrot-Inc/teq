// An abstract type member with parameters applied is bounds-checked like a class (Checking.scala 116-123:
// `tycon.tpe.typeParams`, the member's lambda parameters, the info of its `TypeRef` seen from the prefix), so
// `M[? >: String]` of `type M[X <: Int]` needs `String <: Int`, and `M[String]` too, through a path as well.
// scalac 3.8.4 rejects the three (E057):
//   12:11 Type argument  >: String does not overlap with upper bound Int
//   13:11 Type argument String does not conform to upper bound Int
//   14:17 Type argument String does not conform to upper bound Int
// and accepts `t.M[Int]`. The expectation shows the wildcard as `? >: String`. (Collecting no parameters for
// the application of a type member accepts all of it.)
trait T:
  type M[X <: Int]
  def g: M[? >: String]
  def h: M[String]
def k(t: T): t.M[String] = ???
def l(t: T): t.M[Int] = ???
// expect: 12:12: error: type argument ? >: String does not overlap with upper bound Int
// expect: 13:12: error: type argument String does not conform to upper bound Int
// expect: 14:18: error: type argument String does not conform to upper bound Int
// expect: 3 errors found
