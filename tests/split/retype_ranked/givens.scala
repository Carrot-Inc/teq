package ranked

class Base:
  def name: String = "base"
class Derived extends Base:
  override def name: String = "derived"

/** Two givens of the companion that a search of `Base` ranks: the first one's type names a val whose type is inferred. */
object Base:
  given viaPath: Vals.x.type = Vals.x
  given plain: Base = new Base
