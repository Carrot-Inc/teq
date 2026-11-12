// `Symbol.name` of a symbol a macro names by a string holding a lone surrogate answers its code units
// (the name once read back as U+FFFD).
package mq
import scala.quoted.*
object M {
  inline def names: String = ${ namesImpl }
  def namesImpl(using q: Quotes): Expr[String] = {
    import q.reflect.*
    val a = Symbol.newVal(Symbol.spliceOwner, "a\uD83Db", TypeRepr.of[Int], Flags.EmptyFlags, Symbol.noSymbol)
    val b = Symbol.newVal(Symbol.spliceOwner, "\uD83D", TypeRepr.of[Int], Flags.EmptyFlags, Symbol.noSymbol)
    val c = Symbol.newVal(Symbol.spliceOwner, "\uD83E", TypeRepr.of[Int], Flags.EmptyFlags, Symbol.noSymbol)
    val units = (s: String) => s.map(ch => Integer.toHexString(ch.toInt)).mkString(" ")
    Expr(List(units(a.name), units(b.name), units(c.name), (b.name == c.name).toString).mkString(" | "))
  }
  inline def block: Int = ${ blockImpl }
  def blockImpl(using q: Quotes): Expr[Int] = {
    import q.reflect.*
    val b = Symbol.newVal(Symbol.spliceOwner, "\uD83D", TypeRepr.of[Int], Flags.EmptyFlags, Symbol.noSymbol)
    val c = Symbol.newVal(Symbol.spliceOwner, "\uD83E", TypeRepr.of[Int], Flags.EmptyFlags, Symbol.noSymbol)
    val body = '{ ${ Ref(b).asExprOf[Int] } * 10 + ${ Ref(c).asExprOf[Int] } }.asTerm
    Block(List(ValDef(b, Some(Literal(IntConstant(1)))), ValDef(c, Some(Literal(IntConstant(2))))), body).asExprOf[Int]
  }
}
