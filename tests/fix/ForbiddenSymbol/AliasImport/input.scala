/*
rule = ForbiddenSymbol
ForbiddenSymbol.symbols = ["p.O.T"]
*/
package p
object O:
  type T = Int // assert: ForbiddenSymbol
object Test:
  import O.{T as Renamed}
  val result: Renamed = 1 // assert: ForbiddenSymbol
