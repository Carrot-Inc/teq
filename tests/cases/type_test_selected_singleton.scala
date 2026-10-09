// A `TypeTest` over a singleton type is the identity test with the whole stable path: a selected
// `other.token.type` tests `other`'s token, not the enclosing instance's; a module's member and a
// local keep theirs (TypeTestsCasts.scala 325-326, `isInstance` of the singleton).
import scala.reflect.TypeTest

class Token(val name: String)
class Holder(val name: String):
  val token = new Token(name)
  def test(other: Holder): Unit =
    val tt = summon[TypeTest[Any, other.token.type]]
    println(tt.unapply(other.token).isDefined)
    println(tt.unapply(token).isDefined)
    println(tt.unapply(null).isDefined)
    println((token: Any) match { case _: other.token.type => "other's"; case _ => "mine" })
    println((other.token: Any) match { case _: other.token.type => "other's"; case _ => "mine" })
object Registry:
  val token = new Token("registry")
  object Inner:
    val token = new Token("inner")
object Main:
  def main(args: Array[String]): Unit =
    val a = new Holder("a")
    val b = new Holder("b")
    a.test(b)
    b.test(b)
    val reg = summon[TypeTest[Any, Registry.token.type]]
    println(reg.unapply(Registry.token).isDefined); println(reg.unapply(Registry.Inner.token).isDefined)
    val inner = summon[TypeTest[Any, Registry.Inner.token.type]]
    println(inner.unapply(Registry.Inner.token).isDefined); println(inner.unapply(Registry.token).isDefined)
    val local = new Token("local")
    val loc = summon[TypeTest[Any, local.type]]
    println(loc.unapply(local).isDefined); println(loc.unapply(a.token).isDefined)
