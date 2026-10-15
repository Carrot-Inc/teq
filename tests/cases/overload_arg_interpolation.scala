// An argument typed alone while an overloaded method's alternatives are compared is interpolated as dotty
// adapts it: a variable it holds covariantly is its lower bound, so tyrian's `div(div(br))` types the inner
// `div` at `Html[Nothing]` where the outer one's `Elem[Msg]` takes it.
import scala.annotation.targetName
sealed trait Elem[+M]
sealed trait Html[+M] extends Elem[M]
final case class Tag[+M](name: String, children: List[Elem[M]]) extends Html[M]
object H:
  def br[M](n: Int): Html[M] = Tag("br", Nil)
  @targetName("br-no_attrs") def br[M]: Html[M] = Tag("br", Nil)
  def one[M](children: Elem[M]*): Html[M] = Tag("div", children.toList)
  def div[M](children: Elem[M]*): Html[M] = Tag("div", children.toList)
  @targetName("div-list") def div[M](children: List[Elem[M]]): Html[M] = Tag("div", children)
import H.*
enum Msg:
  case A
@main def run(): Unit =
  val a: Html[Msg] = one(one(br))
  val b: Html[Msg] = div(one(br))
  val c: Html[Msg] = one(div(br))
  val d: Html[Msg] = div(div(br))
  println(List(a, b, c, d).size)
