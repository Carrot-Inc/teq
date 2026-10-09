// expect: 14:22: error: covariant type A occurs in invariant position in type A of variable x
// expect: 16:11: error: covariant type A occurs in contravariant position in type A of parameter a
// expect: 17:22: error: contravariant type A occurs in covariant position in type A of value a
// expect: 19:7: error: contravariant type A occurs in covariant position in type A of method get
// expect: 30:7: error: covariant type T occurs in invariant position in type Cell[T] of class CovCell
// expect: 36:7: error: covariant type A occurs in contravariant position in type A => Unit of method consume
// expect: 39:21: error: type argument String does not conform to upper bound Int
// expect: 41:25: error: type argument Nothing does not conform to lower bound Int
// expect: 46:12: error: type argument String does not conform to upper bound Int
// expect: 48:25: error: type argument String does not conform to upper bound Int
// expect: 10 errors found
// From the conformance report's inference probe t13_variance: variance annotations and the
// bounds of type arguments.
class BadCov[+A](var x: A)
trait BadCov2[+A]:
  def put(a: A): Unit
class BadCon[-A](val a: A)
trait BadCon2[-A]:
  def get: A
trait OkCov[+A]:
  def get: A
  def put[B >: A](b: B): OkCov[B]
trait OkCon[-A]:
  def use(a: A): Unit
class Wrap[+A](val a: A)
class Box[A <: Int](val a: A)
class Lower[A >: Int](val a: A)
trait Cell[T]:
  def get: T
class CovCell[+T](val v: T) extends Cell[T]:
  def get: T = v
trait Container[+A]:
  def all: List[A]
  def first: Option[A]
  def fn: Int => A
  def consume: A => Unit

object V:
  def useBox(b: Box[String]): Int = 1
  def useBox2(b: Box[Int]): Int = b.a
  def useLower(l: Lower[Nothing]): Int = 1
  def useLower2(l: Lower[Any]): Int = 1
  val w: Wrap[Any] = Wrap[Int](1)
  def mk[A <: Int](a: A): Box[A] = Box(a)
  val m1 = mk(1)
  val m2 = mk[String]("a")
  type Alias[A <: Int] = List[A]
  def useAlias(x: Alias[String]): Int = 1

@main def main(): Unit =
  println(V.m1.a)
