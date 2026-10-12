// An extension a trait declares on a class nested in another trait, found lexically in a subclass:
// dotty's reference `C.this.page` is seen from its prefix (`Typer.findRef`, `TypeOps.asSeenFrom`),
// so the parameter `CtrlPage.this.Status` is `C.this.Status`, the type of `Ok` through `C.this`;
// in an object the object's, in an inner class its outer `this`, in an inline method's expansion
// the receiver's.
trait Results:
  class Status(val code: Int):
    def apply(s: String): String = s"$code $s"
  val Ok = new Status(200)
  val NotFound = new Status(404)

trait CtrlPage extends Results:
  extension (s: Status)
    def page(p: String): String = s(p)
    def twice: Status = new Status(s.code * 2)
  def inTrait = Ok.page("trait")

abstract class Base extends Results with CtrlPage

class Clas extends Base:
  def show = Ok.page("x")
  def chained = NotFound.twice.page("chained")
  def viaThis = this.Ok.page("this")
  def direct = page(Ok)("direct")
  def local =
    val st: Status = Ok
    st.page("local")
  class Inner:
    def fromInner = NotFound.page("inner")
  inline def inlined = Ok.page("inline")

object Obj extends Base:
  def show = Ok.page("object")
  class Nested:
    def show = NotFound.page("nested in object")

trait Deep extends CtrlPage
class Deeper extends Deep:
  def show = Ok.twice.twice.page("deeper")

@main def main() =
  val c = new Clas
  println(c.show)
  println(c.chained)
  println(c.viaThis)
  println(c.direct)
  println(c.local)
  println(new c.Inner().fromInner)
  println(c.inlined)
  println(c.inTrait)
  println(Obj.show)
  println(new Obj.Nested().show)
  println(new Deeper().show)
