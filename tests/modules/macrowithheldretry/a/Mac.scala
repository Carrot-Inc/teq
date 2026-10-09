package mra

import scala.quoted.*

// The macro reads the interpolated parts through the std's `FromExpr` of a `StringContext`,
// whose body (`ArrayBuffer.empty`) its run types and expands; the extension's expansion is tried
// more than once, and each run stops at the withheld body and names it, not at an inline call
// of the std's that a retracted attempt left unexpanded.
final case class Address(text: String)

object Mac:
  extension (inline sc: StringContext)
    inline def addr(inline args: Any*): Address = ${ addrImpl('sc) }
  def addrImpl(sc: Expr[StringContext])(using Quotes): Expr[Address] =
    val text = sc.valueOrAbort.parts.mkString
    if Addresses.valid(text) then '{ Address(${ Expr(text) }) }
    else quotes.reflect.report.errorAndAbort(s"not an address: $text")
