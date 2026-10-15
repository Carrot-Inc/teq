// A case class of a block has its companion there, a local object of the block (`Desugar.classDef`'s
// `companionDefs`): the mirror a summon gives (`Synthesizer.companionPath`), its `apply` as a value,
// its extractor, and a `derives` clause's given, each run of the block its own.
import scala.deriving.Mirror
trait Show[A]:
  def show(a: A): String
object Show:
  def derived[A](using m: Mirror.ProductOf[A]): Show[A] = a => "show " + a
def run(n: Int): String =
  case class L(x: Int)
  case class G[A](a: A, n: Int = 7)
  case class S(s: String) derives Show
  val m = summon[Mirror.ProductOf[L]]
  val f: Int => L = L.apply
  val g = G("a")
  val r = L(n) match
    case L(k) if k > 1 => "big"
    case _ => "small"
  s"${m.asInstanceOf[AnyRef] eq L} ${f(n)} $g $r ${summon[Show[S]].show(S("x"))} ${m.fromProduct(Tuple1(n + 1))}"
@main def main(): Unit =
  println(run(1))
  println(run(2))
