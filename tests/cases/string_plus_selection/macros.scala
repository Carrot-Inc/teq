//> using platform js
import scala.quoted.*

object Macros:
  inline def quotedChain(inline a: Any, inline b: String): String = ${ quotedChainImpl('a, 'b) }
  def quotedChainImpl(a: Expr[Any], b: Expr[String])(using Quotes): Expr[String] =
    '{ "".+($a).+($b) }

  inline def quotedToString(inline a: Any, inline b: String): String = ${ quotedToStringImpl('a, 'b) }
  def quotedToStringImpl(a: Expr[Any], b: Expr[String])(using Quotes): Expr[String] =
    '{ "".+($a).toString.+($b) }

  inline def splicedChain(inline a: Any, inline b: String): String = ${ splicedChainImpl('a, 'b) }
  def splicedChainImpl(a: Expr[Any], b: Expr[String])(using Quotes): Expr[String] =
    val chain: Expr[String] = '{ "".+($a) }
    '{ $chain.+($b) }

  inline def reflected(inline a: Any, inline b: String): String = ${ reflectedImpl('a, 'b) }
  def reflectedImpl(a: Expr[Any], b: Expr[String])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val chain = '{ "".+($a) }.asTerm
    val plus = Select.overloaded(chain, "+", Nil, List(b.asTerm))
    plus.asExprOf[String]
