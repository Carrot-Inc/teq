// The fit of a trait's given reached through an object of the implicit scope is kept per object:
// `low` through `S` provides `TC[Pair[S, I]]`, through `I` `TC[Pair[I, S]]`, and each search
// takes the object whose instance fits, whichever was decided first. The same for a type
// member the object fixes (`G.A`) and for a given an object's export routes to another object.
class TC[A](val n: Int)
object TC:
  given fallback: TC[L] = new TC(0)

final class Pair[A, B]

trait Low[A]:
  def id: Int
  given low: TC[A] = new TC(id)

final class S
object S extends Low[Pair[S, I]]:
  def id = 1
final class I
object I extends Low[Pair[I, S]]:
  def id = 2

trait G:
  type A
  def id: Int
  given g: TC[A] = new TC(id)

final class K
object K extends G:
  type A = K
  def id = 3
final class L
object L extends G:
  type A = Int
  def id = 4

trait Base[A]:
  def id: Int
  given base: TC[A] = new TC(id)
final class M
object Impl extends Base[M]:
  def id = 5
object M:
  export Impl.given
  def tag = "m"

@main def main(): Unit =
  val a = summon[TC[Pair[S, I]]].n
  val b = summon[TC[Pair[I, S]]].n
  val c = summon[TC[Pair[S, I]]].n
  val d = summon[TC[L]].n
  val e = summon[TC[K]].n
  val f = summon[TC[L]].n
  val m = summon[TC[M]].n
  println(List(a, b, c, d, e, f, m).mkString(" "))
