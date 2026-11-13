// Two `given T` selectors in one import of an object's body, and a transparent inline method whose
// `summonInline` the first answers at the call in the object: the object's imports keep both bounds, each a
// type. scalac 3.8.4 prints `int`.
trait Ranked[T] { def rank: String }
object Givens {
  given intRanked: Ranked[Int] = new Ranked[Int] { def rank = "int" }
  given text: String = "text"
}
transparent inline def ranked: String = scala.compiletime.summonInline[Ranked[Int]].rank
object Maker {
  import Givens.{given Ranked[?], given String}
  def make(): String = ranked
}
@main def main(): Unit = println(Maker.make())
