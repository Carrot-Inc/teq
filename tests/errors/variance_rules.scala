// expect: 10:8: error: covariant type A occurs in invariant position in type  = A of type X
// expect: 12:7: error: covariant type A occurs in contravariant position in type A of type parameter B
// expect: 15:9: error: covariant type A occurs in contravariant position in type A of parameter a
// expect: 18:7: error: covariant type T occurs in invariant position in type Cell[T] of class CovCell
// expect: 23:7: error: covariant type A occurs in contravariant position in type Option[A => Unit] of method consume
// expect: 30:13: error: covariant type B occurs in contravariant position in type B of parameter b
// expect: 34:11: error: contravariant type A occurs in covariant position in type Sink[A] of parameter g
// expect: 7 errors found
class Alias[+A]:
  type X = A
class Bound[+A]:
  def f[B <: A](b: B): Unit = ()
  def ok[B >: A](b: B): Unit = ()
class Defaulted[+A]:
  def f(a: A = ???): Unit = ()
trait Cell[T]:
  def get: T
class CovCell[+T](val v: T) extends Cell[T]:
  def get: T = v
trait Container[+A]:
  def all: List[A]
  def fn: Int => A
  def consume: Option[A => Unit]
  def either: A | Int
class Plain[+A](x: A):
  def get: A = x
  private def hidden(a: A): Unit = ()
class Both[-A, +B]:
  def apply(a: A): B = ???
  def wrong(b: B): A = ???
trait Sink[-A]:
  def put(a: A): Unit
class Compose[-A, +B](f: Sink[B]):
  def use(g: Sink[A]): Sink[B] = f
@main def run(): Unit = println(Plain(1).get)
