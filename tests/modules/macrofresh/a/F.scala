package mfa

import scala.quoted.*

// Fresh names a macro's reflection makes (`Symbol.freshName`), held by the middle module's
// pickles of the upstream's transparent expansions: a downstream reads them back as the whole
// build names them, and expands the macro at its own site anew (kind 7).
object F:
  transparent inline def twice(inline x: Int): Int = ${ twiceImpl('x) }
  def twiceImpl(x: Expr[Int])(using Quotes): Expr[Int] =
    import quotes.reflect.*
    val name = Symbol.freshName("v")
    val sym = Symbol.newVal(Symbol.spliceOwner, name, TypeRepr.of[Int], Flags.EmptyFlags, Symbol.noSymbol)
    val vd = ValDef(sym, Some(x.asTerm))
    val ref = Ref(sym).asExprOf[Int]
    Block(List(vd), '{ $ref + $ref }.asTerm).asExprOf[Int]
