// Two `given T` selectors in one import of an object's body, one with a wildcard argument, and a summon the
// first answers: the object's imports keep both bounds, each a type. scalac 3.8.4 prints `int`.
trait Ranked[T] { def rank: String }
object Givens {
  given intRanked: Ranked[Int] = new Ranked[Int] { def rank = "int" }
  given text: String = "text"
}
object Maker {
  import Givens.{given Ranked[?], given String}
  def make(): String = summon[Ranked[Int]].rank
}
@main def main(): Unit = println(Maker.make())
