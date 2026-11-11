// `TypeOps.boundsViolations` (TypeOps.scala 606-740) over F-bounded parameters where scalac 3.8.4 finds no violation:
// `skolemizeWildcardArgs`' skolems with `narrowBound`'s GADT bounds for the F-bounded symbol (`isLazyIn`'s carrier)
// through an alias's own F-bound, nested applications, a function type, a result type, by-name and repeated
// parameters, an `extends` clause, match type patterns (`Mode.Pattern`), a given, a using parameter, a higher-kinded
// parameter's `LambdaParam` bound, pos/i6146 through a given and an alias, pos/t2613's shapes, Java's `Enum`, both
// bounds, a constructor's parameters and a typed pattern. The closure adversary of the pass on 5bd85bcd1 (round 23).
trait BS[T, S <: BS[T, S]]
trait IS extends BS[Int, IS]
type Fb[S <: BS[?, S]] = BS[?, S]
def g3[S <: BS[?, S]](x: Fb[S]): Int = 3
def g5[S <: BS[?, S]](x: List[BS[?, S]]): Int = 5
def g6[S <: BS[?, S]](x: BS[?, S] => Int): Int = 6
def g7[S <: BS[?, S]]: Option[BS[?, S]] = None
def g8[S <: BS[?, S]](x: => BS[?, S], y: BS[?, S]*): Int = 8
trait Holder[A]
class E9[S <: BS[?, S]] extends Holder[BS[?, S]]
type M11[X] = X match
  case BS[?, s] => s
  case _ => Nothing
type M12[X] = X match
  case BS[? <: Int, s] => s
  case _ => Nothing
trait Conv[A]
given c13[S <: BS[?, S]]: Conv[BS[?, S]] = new Conv[BS[?, S]] {}
def f14[S <: BS[?, S]](using x: Conv[BS[?, S]]): Int = 14
def f16[F[X <: BS[?, X]]](x: F[IS]): Int = 16
sealed trait BSElem[T, S <: BS[?, S]]
object BSElem:
  given intStreamShape: BSElem[Int, IS] = new BSElem[Int, IS] {}
class Ops[A]:
  def asJavaSeqStream[S <: BS[?, S]](using s: BSElem[A, S]): Int = 21
type Shape[T, S <: BS[?, S]] = BSElem[T, S]
class Ops2[A]:
  def asJavaSeqStream[S <: BS[?, S]](implicit s: Shape[A, S]): Int = 22
class Row
abstract class MyRelation[R <: Row, Relation <: MyRelation[R, Relation]]
type M23 = MyRelation[? <: Row, ? <: MyRelation[?, ?]]
abstract class MyRelation2[R <: Row, +Relation <: MyRelation2[R, Relation]]
type M24 = MyRelation2[? <: Row, ? <: MyRelation2[Row, ?]]
def g26[E <: java.lang.Enum[E]](x: java.lang.Enum[? <: E]): Int = 26
def h26(x: java.lang.Enum[?]): Int = x.ordinal
def g38[S >: IS <: BS[?, S]](x: BS[?, S]): Int = 38
def g39[S <: BS[?, S], R <: BS[?, S]](x: R): Int = 39
class K41[S <: BS[?, S]](val x: List[BS[?, S]])
def g42(x: Any): Int = x match
  case b: BS[?, ?] => 42
  case _ => 0
object IS1 extends IS
@main def run(): Unit =
  println(List(g3[IS](IS1), g5[IS](Nil), Ops[Int]().asJavaSeqStream, g26(java.util.concurrent.TimeUnit.SECONDS)))
  println(List(g42(IS1), g42(1), f16[[X <: BS[?, X]] =>> Int](0)))
