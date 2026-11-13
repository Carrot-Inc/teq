// A `given T` selector at the top of a file whose signatures name nothing (Maker.scala), so that the file's
// imports are first read where a body of it is typed, and a plain inline method whose `summonInline` is
// answered where the inlining phase expands the call. The selector's type is one no given of the object has:
// nothing is imported. scalac 3.8.4: E172, no given instance of type Ranked[String].
trait Ranked[T] { def rank: String }
object Givens {
  given intRanked: Ranked[Int] = new Ranked[Int] { def rank = "int" }
  given strRanked: Ranked[String] = new Ranked[String] { def rank = "str" }
}
object Lib { inline def plainRanked[A]: String = scala.compiletime.summonInline[Ranked[A]].rank }
@main def main(): Unit = println(Maker.make())
