/*
rule = ForbiddenSymbol
ForbiddenSymbol.symbols = ["p.sub.package.f"]
*/
package p
package object sub:
  def f(x: Int): Int = x // assert: ForbiddenSymbol
object Test:
  import sub.{f as renamed} // assert: ForbiddenSymbol
  val a = renamed(1) // assert: ForbiddenSymbol
  val b = sub.f(2) // assert: ForbiddenSymbol
