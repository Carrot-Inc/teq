// An old-style abstract given, `given x: T` with nothing after the type, is an abstract def flagged
// `Given` (dotty's `Parsers.givenDef`): a given of the trait's body, implemented without `override`
// by a given, a def, a val or an implicit def of the class, with parameters and type parameters as
// a def's. teq read it as a given object extending `T` ("this type cannot be extended").
trait T:
  given x: Int
  def twice = summon[Int] * 2
  def next = x + 1

class ByGiven extends T:
  given x: Int = 5
class ByOverride extends T:
  override given x: Int = 6
class ByDef extends T:
  def x: Int = 7
class ByVal extends T:
  val x: Int = 8
class ByImplicit extends T:
  implicit def x: Int = 9

abstract class A { given y: String }
object B extends A { given y: String = "b" }

trait P:
  given show(using i: Int): String
  given ord[A](using o: Ordering[A]): Ordering[List[A]]
class Q extends P:
  given show(using i: Int): String = "s" + i
  given ord[A](using o: Ordering[A]): Ordering[List[A]] = Ordering.Implicits.seqOrdering[List, A]

@main def run(): Unit =
  for t <- List(ByGiven(), ByOverride(), ByDef(), ByVal(), ByImplicit()) do println(s"${t.x} ${t.twice} ${t.next}")
  println(B.y)
  println(summon[String](using B.y))
  val q = Q()
  given Int = 3
  println(q.show)
  import q.given
  println(summon[String])
  println(List(List(2), List(1)).sorted)
