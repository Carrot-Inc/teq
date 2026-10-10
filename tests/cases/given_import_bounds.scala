// A `given T` selector admits the givens of its clause that match the union of its selectors' types
// (`Namer.importBound`, `matchesImportBound`), a generic given instantiated, an unbounded one all.
trait Ord[T] { def name: String }
object O {
  given intOrd: Ord[Int] = new Ord[Int] { def name = "int" }
  given listOrd[T](using o: Ord[T]): Ord[List[T]] = new Ord[List[T]] { def name = "list " + o.name }
  given text: String = "text"
  given n: Int = 3
}
object Q {
  import O.{given Ord[?]}
  def a = summon[Ord[List[Int]]].name
}
object R {
  import O.{given Int, given String}
  def b = summon[String] + summon[Int]
}
object S {
  import O.{given Int, given}
  def c = summon[String]
}
@main def run(): Unit = println(Q.a + " " + R.b + " " + S.c)
