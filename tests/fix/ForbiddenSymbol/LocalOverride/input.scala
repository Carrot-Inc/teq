/*
rule = ForbiddenSymbol
ForbiddenSymbol.symbols = ["p.Base.last"]
*/
package p
trait Base:
  def last: Int = 1 // assert: ForbiddenSymbol
object Test:
  def run: Int =
    class Local extends Base:
      override def last: Int = 2
    new Local().last // assert: ForbiddenSymbol
