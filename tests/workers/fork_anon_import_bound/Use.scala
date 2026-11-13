// A `given T` selector in a block that holds an anonymous class: the class's environment keeps the import with
// its bound, a type. scalac 3.8.4 prints `int`.
trait Ranked[T] { def rank: String }
object Givens {
  given intRanked: Ranked[Int] = new Ranked[Int] { def rank = "int" }
  given text: String = "text"
}
trait Shown { def show: String }
object Maker {
  def make(): Shown = {
    import Givens.{given Ranked[?]}
    new Shown { def show = summon[Ranked[Int]].rank }
  }
}
@main def main(): Unit = println(Maker.make().show)
