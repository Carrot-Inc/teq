// A case class below a class takes the superclass's `toString` where it defines one (dotty's
// `existingDef`) and `Product`'s concrete members as any; so does an enum extending a class.
abstract class Base:
  override def toString: String = "base"
case class C(x: Int, y: String) extends Base
case class D(x: Int) extends Base:
  override def productPrefix: String = "Dd"
enum E(val k: Int):
  case A extends E(1)
  case B(z: Int) extends E(2)
@main def run(): Unit =
  println(C(1, "a"))
  println((C(1, "a"): Product).productIterator.toList)
  println(D(2).productPrefix)
  println(C(1, "a") == C(1, "a"))
  println((E.B(3): Product).productIterator.toList)
  println(E.A.productArity)
