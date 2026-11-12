/*
rule = ForbiddenSymbol
ForbiddenSymbol.symbols = [
  "localdata.Base.last"
]
*/
package localdata

// A symbol of a package whose name begins with `local` is a global one: `localdata/Sub#last().` overrides the entry.
class Base:
  def last: Int = 1 // assert: ForbiddenSymbol

class Sub extends Base:
  override def last: Int = 2

object LocalNamedPackage:
  val overrider = new Sub().last // assert: ForbiddenSymbol
  val own = new Base().last // assert: ForbiddenSymbol
