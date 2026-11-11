// A bounded wildcard is accepted when its ends overlap its parameter's bounds (`checkOverlapsBounds`, TypeOps.scala
// 700-701), even where the written and declared bounds have only `Nothing` in common (`Up[? <: String]` for
// `T <: Int`), where the wildcard's own ends conflict (`? >: String <: Int` of an unbounded parameter), and for a path's
// abstract lower end (`t.U <: Int`). scalac 3.8.4 accepts all of it.
class Up[T <: Int](val n: Int)
class Lo[T >: String](val n: Int)
class Both[T >: Null <: AnyRef](val n: Int)
class Free[T](val n: Int)
class Sing[T <: Singleton](val n: Int)
class Ref[T <: AnyRef](val n: Int)
trait Path:
  type U <: Int
type A = Up[? <: String]
type B = Lo[? >: Int]
type C = Both[? >: Null <: String]
type D = Free[? >: String <: Int]
type E = Up[? >: Int & String]
type F = Ref[? >: Null]
type G = Sing[? >: 1]
def fromPath(t: Path): Up[? >: t.U] = Up[t.U](7)
@main def main(): Unit =
  val a: A = Up[Nothing](1)
  val b: B = Lo[Any](2)
  val c: C = Both[String](3)
  val d: D = null
  val e: E = Up[Int](5)
  val f: F = Ref[String](6)
  val g: G = Sing[1](8)
  println(List(a.n, b.n, c.n, d == null, e.n, f.n, g.n, fromPath(new Path { type U = 3 }).n).mkString(","))
