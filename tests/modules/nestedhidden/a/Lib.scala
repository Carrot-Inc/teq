package nestedhidden

// An abstract redeclaration in a base nested in an object hides the ancestor's body, through the
// products as from source (dotty's `existingDef` looks at every base).
object Scope:
  trait Base:
    override def toString: String = "ancestor"
    def productPrefix: String = "ancestor-prefix"
  trait Abstract extends Base:
    override def toString: String
    override def productPrefix: String
  case class Item(x: Int) extends Abstract
