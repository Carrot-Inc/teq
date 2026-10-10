// A `given T` selector reads the givens of the object it imports from as that object sees them:
// `box` of `Lib[A]` is a `Box[String]` in `Strings` (`matchesImportBound` on the member as seen
// from the qualifier), and a generic given matches where an instance of it does.
trait Box[A] { def value: Int }
trait Lib[A] { given box: Box[A] with { def value = 1 } }
object Strings extends Lib[String]
object Generic { given any[A]: Box[A] with { def value = 2 } }
object Main {
  import Strings.{given Box[String]}
  def first: Int = summon[Box[String]].value
}
object Other {
  import Generic.{given Box[String]}
  def second: Int = summon[Box[Int]].value
}
@main def run(): Unit = println(Main.first + Other.second)
