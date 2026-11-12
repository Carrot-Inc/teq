// A macro whose declared result type depends on a parameter (`tag.Out`), and whose implementation
// constructs the class of Scale.scala, the program's smallest file, which holds no quoted code.
package own

import scala.quoted.*

trait Tag:
  type Out
object IntTag extends Tag:
  type Out = Int

object Base:
  inline def dep(inline n: Int)(tag: Tag): tag.Out = ${ depImpl[tag.Out]('n) }
  def depImpl[T: Type](n: Expr[Int])(using Quotes): Expr[T] =
    val s = new Scale(1)
    Expr(s.twice(n.valueOrAbort)).asInstanceOf[Expr[T]]
