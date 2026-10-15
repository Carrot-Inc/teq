// A case class nested in a class has its companion, an object of each instance, as its mirror
// (`SymUtils.useCompanionAsProductMirror` 166, no owner's restriction), read through the type's
// prefix (`Synthesizer.companionPath` 304): two summons are one object, the companion itself.
import scala.deriving.Mirror
class Outer:
  case class C(x: Int)
  def test(): Unit =
    val a = summon[Mirror.ProductOf[C]]
    val b = summon[Mirror.ProductOf[C]]
    println(a.asInstanceOf[AnyRef] eq b.asInstanceOf[AnyRef])
    println(a.asInstanceOf[AnyRef] eq C)
    println(a.fromProduct(Tuple1(3)))
@main def run(): Unit =
  val o = new Outer()
  o.test()
  val m = summon[Mirror.ProductOf[o.C]]
  println(m.asInstanceOf[AnyRef] eq o.C)
  println(m.fromProduct(Tuple1(4)))
