// An upstream's macros whose quotes call another macro with an inline argument that is an inline call itself
// (`'{ M.m(M.k) + 1 }`), or with its inline parameter's default left out (`'{ M.d() + 1 }`, the default's getter an
// inline method of its own, Desugar.scala:429), directly and through an export forwarder: the downstream's expansion
// expands the argument where the second macro's quote splices it (`'{ $x * 2 }`), once, an expanded call being no
// inline call (`Inlines.needsInlining`).
package mia

import scala.quoted.*

object M:
  inline def k: Int = 3
  inline def m(inline x: Int): Int = ${ twice('x) }
  inline def d(inline x: Int = 4): Int = ${ twice('x) }
  def twice(x: Expr[Int])(using Quotes): Expr[Int] = '{ $x * 2 }
  inline def viaArg: Int = ${ viaArgImpl }
  def viaArgImpl(using Quotes): Expr[Int] = '{ M.m(M.k) + 1 }
  inline def viaDefault: Int = ${ viaDefaultImpl }
  def viaDefaultImpl(using Quotes): Expr[Int] = '{ M.d() + 1 }
  inline def viaForwarder: Int = ${ viaForwarderImpl }
  def viaForwarderImpl(using Quotes): Expr[Int] = '{ Fw.d() + 2 }

object Fw:
  export M.d
