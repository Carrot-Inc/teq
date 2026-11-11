import scala.quoted.*

object Macros:
  def wordsImpl(sc: Expr[StringContext])(using Quotes): Expr[Int] =
    import quotes.reflect.*
    val parts = sc match
      case '{ StringContext(${ Varargs(ps) }*) } => ps.map(_.valueOrAbort)
      case _ => report.errorAndAbort("words needs a literal", sc)
    Expr(parts.mkString(" ").split(" ").length)

extension (inline sc: StringContext)
  inline def words(inline args: Any*): Int = ${ Macros.wordsImpl('sc) }
