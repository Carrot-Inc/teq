// The design pass's witnesses of lines the task list still held, which teq already agrees with scalac on:
// an applied type variable's argument inferred through a covariant parent (`id(new Box)` as an `F`), a
// `String`'s `CharSequence` and `Serializable` tests, an array class's `isInstance`, a program's own
// `ClassTag`, `Tuple`'s entry points and `CanEqual` givens, a generic class's type test over a value of that
// class (true for an instance, false for `null`) and over `Any`.
import scala.language.strictEquality
import scala.reflect.ClassTag

trait F[+E, +A]
class Box[E, A] extends F[E, A]
def id[G[_], A](x: G[A]): G[A] = x
val inferred: F[String, Int] = id(new Box[String, Int])

val tag = new ClassTag[String]:
  def runtimeClass: Class[?] = classOf[String]

def tupleEq[H1, T1 <: Tuple, H2, T2 <: Tuple](using CanEqual[H1, H2], CanEqual[T1, T2]): CanEqual[H1 *: T1, H2 *: T2] =
  summon[CanEqual[H1 *: T1, H2 *: T2]]
val tuple1Eq = summon[CanEqual[Tuple1[Int], Tuple1[Int]]]
def first[T <: NonEmptyTuple](x: T) = x.head

class Cell[T]
def check[T](x: Cell[T]): Boolean = x match
  case _: Cell[T] => true
  case _ => false
def any(x: Any): Boolean = x match
  case _: Cell[Int] @unchecked => true
  case _ => false

@main def main(): Unit =
  println(inferred.isInstanceOf[Box[?, ?]])
  println(("x": Any).isInstanceOf[CharSequence])
  println(("x": Any).isInstanceOf[java.io.Serializable])
  println(classOf[Array[Int]].isInstance(Array(1)))
  println(tag.runtimeClass.getName)
  println(Tuple(1))
  println(Tuple.unapply(EmptyTuple))
  println(first((2, "b")))
  println(check(new Cell[Int]))
  println(check(null))
  println(any("wrong"))
