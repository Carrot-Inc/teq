import scala.quoted.*

class Holder(val name: String, val items: Array[Int], val f: Int => Int)

object Kept:
  val holder = new Holder("kept", Array(1, 2, 3), x => x + 1)

object M:
  def kept(using Quotes): Expr[Int] = Expr(System.identityHashCode(Kept.holder))
  def site(using Quotes): Expr[Int] =
    import quotes.reflect.*
    val p = Position.ofMacroExpansion
    Expr(System.identityHashCode(new Holder(p.sourceFile.name, Array(p.start, p.end), x => x * 2)))

inline def keptHash: Int = ${ M.kept }
inline def siteHash: Int = ${ M.site }
