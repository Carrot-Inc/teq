/*
rule = ForbiddenSymbol
ForbiddenSymbol.symbols = ["p.O.ext"]
*/
package p
object O:
  extension (x: Int) def ext: Int = x + 1 // assert: ForbiddenSymbol
object Test:
  import O.{ext as renamed} // assert: ForbiddenSymbol
  val result = 1.renamed // assert: ForbiddenSymbol
