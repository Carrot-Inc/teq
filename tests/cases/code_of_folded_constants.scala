// `codeOf` shows its argument as the typer typed it (`Tree.show`): an operation ConstFold gave a constant type as
// written, a constant member behind its stable path as that selection, its prefix as the printer shows an identifier's,
// a transparent call as its expansion after its bindings, and inside an inline method's body the literal its InlineTyper makes of them.
// scalac prints the lines of the .expected file.
import scala.compiletime.codeOf
object K:
  final val two = 2
  transparent inline def tr: Int = 5
  transparent inline def trp(): Int = 6
  transparent inline def pick(inline b: Boolean): Int = inline if b then 1 else 2
  transparent inline def twice(x: Int): Int = x * 2
  transparent inline def blk: Int = { val q = 4; q + 1 }
  transparent inline def lz: Int = { lazy val l = 5; l + 0 }
  object In:
    final val three = 3
  def show: String = codeOf(two)
class C:
  final val t = 7
  def show: String = codeOf(t)
  def showSelf: String = codeOf(this.t)
def eff(): Int = 3
inline def folded: String = codeOf(K.two * 3)
inline def ref: String = codeOf(K.two)
@main def run(): Unit =
  println(codeOf(1 + 1))
  println(codeOf("a" + "b"))
  println(codeOf(-(1)))
  println(codeOf(K.two))
  println(codeOf(K.two * 3))
  println(codeOf(K.In.three))
  println(codeOf(Int.MaxValue))
  println(K.show)
  println(C().show)
  println(C().showSelf)
  println(folded)
  println(ref)
  println(List(codeOf(K.tr), codeOf(K.trp()), codeOf(K.pick(true)), codeOf(K.tr * K.two)))
  println(codeOf(K.twice(eff())))
  println(codeOf(K.blk))
  println(codeOf(K.lz))
  println(codeOf({ var w = 1; w = 2; w + 0 }))
