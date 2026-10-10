// A NotGiven candidate is negated inside the ranked search (`negateIfNot(tryImplicit(..))`): a nearer
// import's candidate of the same name hides the outer one, whose failing using clause is never tried, and
// the nearer one's success negates to a failure, so the companion's fallback decides.
import scala.util.NotGiven
import scala.compiletime.summonFrom
trait Missing
object Outer:
  given nf(using Missing): NotGiven[Int] = ???
object Inner:
  given nf: NotGiven[Int] = ???
inline def ask = summonFrom { case _: NotGiven[Int] => "absent"; case _ => "present" }
@main def run(): Unit =
  given Int = 1
  import Outer.given
  locally {
    import Inner.given
    println(ask)
  }
