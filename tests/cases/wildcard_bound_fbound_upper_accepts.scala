// An upper-bounded or unbounded wildcard of an F-bounded parameter is accepted: its lower end `Nothing` is below any
// bound, whatever its upper end (`D[? <: String]`), including Java's `Enum[E <: Enum[E]]`, `Comparable` F-bounds and
// scala3's tests/pos/t2613.scala (`MyRelation[_ <: Row, _ <: MyRelation[_, _]]`) and an F-bound over a wildcard
// in a parameter's own bound (tests/pos/i6146.scala's shape). scalac 3.8.4 accepts all of it.
import java.util.concurrent.TimeUnit
class D[T <: D[T]](val n: Int)
class E extends D[E](1)
class Cmp[T <: Comparable[T]](val n: Int)
class Row
abstract class MyRelation[R <: Row, +Relation <: MyRelation[R, Relation]]
trait BS[T, S <: BS[T, S]]
sealed trait E1[S <: BS[? <: Int, S]]
sealed trait E2[S <: BS[? >: Int, S]]
type X = D[? <: String]
type Y = D[? <: D[?]]
type Z = D[? >: Nothing <: D[? <: D[?]]]
type M = MyRelation[? <: Row, ? <: MyRelation[?, ?]]
def unit(t: java.lang.Enum[? <: TimeUnit]): String = t.name
def any(t: java.lang.Enum[?]): Int = t.ordinal
@main def main(): Unit =
  val x: X = null
  val y: Y = new E
  val c: Cmp[? <: String] = Cmp[String](2)
  val m: M = null
  val enums: List[java.lang.Enum[? <: TimeUnit] => String] = List(unit)
  println(List(x == null, y.n, c.n, m == null, enums.size).mkString(","))
