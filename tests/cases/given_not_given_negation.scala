// A `NotGiven` search negates each candidate's result (`Implicits.negateIfNot`): a program's own candidate whose
// using clause has no instance is a success, `NotGiven.value`, whether `T` has a given or not; without one, the
// library's candidates make it the negation of the search for `T`.
import scala.compiletime.summonFrom
import scala.util.NotGiven
trait Missing
object Present:
  given Int = 1
  given nf(using Missing): NotGiven[Int] = ???
object Absent:
  given nf(using Missing): NotGiven[Int] = ???
inline def ask: String = summonFrom { case _: NotGiven[Int] => "not given"; case _ => "given" }
@main def run(): Unit =
  locally { import Present.given; println(summon[NotGiven[Int]] != null) }
  locally { import Absent.given; println(summon[NotGiven[Int]] != null) }
  locally { given Int = 2; println(ask) }
  println(ask)
