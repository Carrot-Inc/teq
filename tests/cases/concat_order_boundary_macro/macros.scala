//> using platform js
import scala.quoted.*

object Macros:
  inline def built(): String = ${ builtImpl }
  def builtImpl(using Quotes): Expr[String] =
    val sb = new java.lang.StringBuilder("a")
    def add(): String =
      sb.append("b")
      ""
    def reset(): Unit = sb.setLength(1)
    val plain = ("" + sb) + add()
    reset()
    val str = ("" + sb).toString + add()
    reset()
    val asc = (("" + sb): String) + add()
    reset()
    val cast = ("" + sb).asInstanceOf[String] + add()
    reset()
    val cif = (if true then "" + sb else "") + add()
    reset()
    val nif = (if sb.length == 1 then "" + sb else "") + add()
    Expr(plain + "," + str + "," + asc + "," + cast + "," + cif + "," + nif)

  inline def viaToString(inline a: Any, inline b: String): String = ${ viaToStringImpl('a, 'b) }
  def viaToStringImpl(a: Expr[Any], b: Expr[String])(using Quotes): Expr[String] =
    '{ ("" + $a).toString + $b }

  inline def viaAscription(inline a: Any, inline b: String): String = ${ viaAscriptionImpl('a, 'b) }
  def viaAscriptionImpl(a: Expr[Any], b: Expr[String])(using Quotes): Expr[String] =
    '{ (("" + $a): String) + $b }

  inline def viaCast(inline a: Any, inline b: String): String = ${ viaCastImpl('a, 'b) }
  def viaCastImpl(a: Expr[Any], b: Expr[String])(using Quotes): Expr[String] =
    '{ ("" + $a).asInstanceOf[String] + $b }

  inline def viaConstantIf(inline a: Any, inline b: String): String = ${ viaConstantIfImpl('a, 'b) }
  def viaConstantIfImpl(a: Expr[Any], b: Expr[String])(using Quotes): Expr[String] =
    '{ (if true then "" + $a else "") + $b }

  inline def viaIf(inline a: Any, inline b: String): String = ${ viaIfImpl('a, 'b) }
  def viaIfImpl(a: Expr[Any], b: Expr[String])(using Quotes): Expr[String] =
    '{ (if $a != null then "" + $a else "") + $b }

  inline def castSplice(inline a: Any, inline b: String): String = ${ castSpliceImpl('a, 'b) }
  def castSpliceImpl(a: Expr[Any], b: Expr[String])(using Quotes): Expr[String] =
    val chain: Expr[String] = '{ "" + $a }
    '{ $chain.asInstanceOf[String] + $b }

  inline def plainSplice(inline a: Any, inline b: String): String = ${ plainSpliceImpl('a, 'b) }
  def plainSpliceImpl(a: Expr[Any], b: Expr[String])(using Quotes): Expr[String] =
    val chain: Expr[String] = '{ "" + $a }
    '{ $chain + $b }

  inline def ascribedSplice(inline a: Any, inline b: String): String = ${ ascribedSpliceImpl('a, 'b) }
  def ascribedSpliceImpl(a: Expr[Any], b: Expr[String])(using Quotes): Expr[String] =
    val chain: Expr[String] = '{ "" + $a }
    '{ ($chain: String) + $b }

  inline def inlineCallInQuote(inline a: Any, inline b: String): String = ${ inlineCallInQuoteImpl('a, 'b) }
  def inlineCallInQuoteImpl(a: Expr[Any], b: Expr[String])(using Quotes): Expr[String] =
    '{ shown($a) + $b }

  inline def shown(x: Any): String = "in:" + x

  inline def reflected(inline a: Any, inline b: String): String = ${ reflectedImpl('a, 'b) }
  def reflectedImpl(a: Expr[Any], b: Expr[String])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val chain = '{ "" + $a }.asTerm
    val typed = Typed(chain, TypeTree.of[String])
    val plus = Select.unique(typed, "+")
    Apply(plus, List(b.asTerm)).asExprOf[String]

  inline def reflectedIf(inline a: Any, inline b: String): String = ${ reflectedIfImpl('a, 'b) }
  def reflectedIfImpl(a: Expr[Any], b: Expr[String])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val chain = '{ "" + $a }.asTerm
    val cond = If(Literal(BooleanConstant(true)), chain, Literal(StringConstant("")))
    val plus = Select.unique(cond, "+")
    Apply(plus, List(b.asTerm)).asExprOf[String]

  inline def reflectedToString(inline a: Any, inline b: String): String = ${ reflectedToStringImpl('a, 'b) }
  def reflectedToStringImpl(a: Expr[Any], b: Expr[String])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val chain = '{ "" + $a }.asTerm
    val shown = Apply(Select.unique(chain, "toString"), Nil)
    Apply(Select.unique(shown, "+"), List(b.asTerm)).asExprOf[String]

  inline def reflectedPlain(inline a: Any, inline b: String): String = ${ reflectedPlainImpl('a, 'b) }
  def reflectedPlainImpl(a: Expr[Any], b: Expr[String])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val chain = '{ "" + $a }.asTerm
    Apply(Select.unique(chain, "+"), List(b.asTerm)).asExprOf[String]

  inline def reflectedBlock(inline a: Any, inline b: String): String = ${ reflectedBlockImpl('a, 'b) }
  def reflectedBlockImpl(a: Expr[Any], b: Expr[String])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val chain = '{ "" + $a }.asTerm
    val block = Block(List('{ println("  stat") }.asTerm), chain)
    Apply(Select.unique(block, "+"), List(b.asTerm)).asExprOf[String]

  inline def asExprOfString(inline a: Any, inline b: String): String = ${ asExprOfStringImpl('a, 'b) }
  def asExprOfStringImpl(a: Expr[Any], b: Expr[String])(using Quotes): Expr[String] =
    val chain: Expr[Any] = '{ "" + $a }
    val s = chain.asExprOf[String]
    '{ $s + $b }

  inline def twice(inline a: Any, inline b: String): String = ${ twiceImpl('a, 'b) }
  def twiceImpl(a: Expr[Any], b: Expr[String])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val chain = '{ "" + $a }
    val typed = Typed(chain.asTerm, TypeTree.of[String]).asExprOf[String]
    '{ $typed + $b + ($chain + $b) }
