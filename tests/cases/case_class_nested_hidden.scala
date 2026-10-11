// An abstract redeclaration in a base nested in an object hides the ancestor's body as one at top
// level does (dotty's `existingDef` looks at every base): the rule gives `toString` and
// `productPrefix` again.
object Scope:
  trait Base:
    override def toString: String = "ancestor"
  trait Abstract extends Base:
    override def toString: String
  case class Item(x: Int) extends Abstract

object Nest:
  trait Root:
    override def toString: String = "hidden"
    def productPrefix: String = "hidden-prefix"
  trait Deferred extends Root:
    override def toString: String
    override def productPrefix: String
  case class Fresh(n: Int) extends Deferred

@main def run(): Unit =
  println(Scope.Item(3))
  println(Nest.Fresh(12))
  println((Nest.Fresh(12): Product).productPrefix)
