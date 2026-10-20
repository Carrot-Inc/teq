// An import's path must be realizable (`Checking.checkRealizable`, `CheckRealizable.realizability`):
// its type's type members, the classes' type parameters among them, are read as seen from the
// type, an intersection's merged (`boundsRealizability`: `nonClassTypeMembers`, the refinements'
// names, `tp.member(name)`, `Denotations.infoMeet`'s `TypeBounds.&`), each with `lo <: hi`, the
// first problem reported.
// expect: 17:30: error: (Test.b : Box[String, Int]) is not a legal path since its underlying type (Test.b : Box[String, Int]) has a member X with possibly conflicting bounds String <: ... <: Int
// expect: 18:30: error: (Test.r : Ref{type X >: String} & Ref{type X <: Int}) is not a legal path since its underlying type (Test.r : Ref{type X >: String} & Ref{type X <: Int}) has a member X with possibly conflicting bounds String <: ... <: Int
// expect: 19:29: error: (Test.o : One[String] & One[Int]) is not a legal path since its underlying type (Test.o : One[String] & One[Int]) has a member A with possibly conflicting bounds String | Int <: ... <: String & Int
// expect: 3 errors found
trait Box[A, B] { type X >: A <: B; val x: Int }
trait Ref { type X; val x: Int }
trait One[A] { val x: Int }
object Test {
  final lazy val b: Box[String, Int] = ???
  final lazy val r: (Ref { type X >: String }) & (Ref { type X <: Int }) = ???
  final lazy val o: One[String] & One[Int] = ???
  def bounds: Int = { import b.*; x }
  def merged: Int = { import r.*; x }
  def bases: Int = { import o.*; x }
}
