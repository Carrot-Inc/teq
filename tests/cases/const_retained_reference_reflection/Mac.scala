// A macro reading, with retained trees, the bodies of a provider unit the inlining round has folded: Symbol.tree
// gives the trees as they were typed, before FirstTransform (Pickler and defTree before FirstTransform,
// Compiler.scala:50, 52, 63), so `c.x` and `Lib.y` stay references. From Codex's review of tr/cut-head-tail-c2 6689715658.
import scala.quoted.*
class C:
  val x: 42 = 42
object Lib:
  def read(c: C): Int = c.x
  val y: 43 = 43
  def readY: Int = y
object Mac:
  inline def show: String = ${ impl }
  def impl(using Quotes): Expr[String] =
    import quotes.reflect.*
    val lib = TypeRepr.of[Lib.type].typeSymbol
    val read = lib.declaredMethod("read").head
    val readY = lib.declaredMethod("readY").head
    Expr(read.tree.show + "\n" + readY.tree.show)
