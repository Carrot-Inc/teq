// A given a trait declares at its type parameter, `given value: T` of a `G[T <: Base]`, read
// through the companions extending it: as seen from `object A extends G[A]` it is an `A`, so the
// class of the parameter's bound does not rule it out (dotty's `eligible` filters a candidate as
// its prefix sees it). For a union both companions' are eligible, and the more general one wins.
trait Base:
  def tag: String
class A extends Base:
  def tag = "A"
class B extends A:
  override def tag = "B"
trait G[T <: Base]:
  def make: T
  given value: T = make
object A extends G[A]:
  def make = new A
object B extends G[B]:
  def make = new B
@main def run(): Unit =
  println(summon[A].tag)
  println(summon[B].tag)
  println(summon[A | B].tag)
