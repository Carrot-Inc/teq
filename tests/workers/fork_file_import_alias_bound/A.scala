// As fork_file_import_bound, the selector's type written through a match alias seen through a prefix
// (`Kind[Int]#Elem[Int]`, which is Int): the import brings the given of Ranked[Int] and the String, not the
// one the call asks for. scalac 3.8.4: E172, no given instance of type Ranked[String].
trait Ranked[T] { def rank: String }
class Kind[T] { type Elem[X] = X match { case Int => T; case _ => X } }
object Givens {
  given strRanked: Ranked[String] = new Ranked[String] { def rank = "str" }
  given intRanked: Ranked[Int] = new Ranked[Int] { def rank = "int" }
  given text: String = "text"
}
object Lib { inline def plainRanked[A]: String = scala.compiletime.summonInline[Ranked[A]].rank }
@main def main(): Unit = println(Maker.make())
