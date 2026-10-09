// Compiled with Scala 3.3.8 (TASTy 28.3), as src/quotepat.scala is: holes in the arguments of
// calls, of a curried call and as an extractor, in the `ExprMatch` pickling of quote patterns,
// and a hole ascribed a type variable, whose binder `a: Expr[t]` is no `Type[t]` given
// (`tests/classpath/js/quote_patterns_28_3_calls.scala`).
package fix.qpat

import scala.quoted.*

object QpFns:
  def single(a: Int): Int = a
  def pair(a: Int)(b: String): String = s"$a/$b"
  def many(xs: Int*): Int = xs.sum

object QpProbe:
  inline def one(inline e: Any): String = ${ oneImpl('e) }
  def oneImpl(e: Expr[Any])(using Quotes): Expr[String] = e match
    case '{ QpFns.single($a) } => Expr("single " + a.valueOrAbort)
    case _ => Expr("other")

  inline def curried(inline e: Any): String = ${ curriedImpl('e) }
  def curriedImpl(e: Expr[Any])(using Quotes): Expr[String] = e match
    case '{ QpFns.pair($a)($b) } => Expr("pair " + a.valueOrAbort + " " + b.valueOrAbort)
    case _ => Expr("other")

  inline def spread(inline e: Any): String = ${ spreadImpl('e) }
  def spreadImpl(e: Expr[Any])(using Quotes): Expr[String] = e match
    case '{ QpFns.many(${Varargs(xs)}*) } => Expr("many " + xs.map(_.valueOrAbort).mkString(","))
    case _ => Expr("other")

  inline def kind(inline e: Any): String = ${ kindImpl('e) }
  def kindImpl(e: Expr[Any])(using Quotes): Expr[String] = e match
    case '{ $s: String } => Expr("string " + s.valueOrAbort)
    case '{ $a: t } => Expr("other " + Type.show[t] + " " + a.isExprOf[Int])
