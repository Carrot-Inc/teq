// A class parameter marked `implicit` alone, after a plain one (Laminar's `MountContext(thisNode: El, implicit val
// owner: Owner)`): its accessor is an implicit member, a given of the class body and of an import of the
// instance's members, as dotty's `param` reads any modifier of a class parameter (`Parsers.termParamClause`).
class Owner(val name: String)
class Ctx(val node: String, implicit val owner: Owner):
  def ownerName(using o: Owner): String = o.name
  def run: String = s"$node in ${ownerName}"

def named(using o: Owner): String = o.name

@main def main(): Unit =
  val ctx = new Ctx("div", new Owner("root"))
  println(ctx.run)
  import ctx.*
  println(named)
  println(ctx.owner.name)
