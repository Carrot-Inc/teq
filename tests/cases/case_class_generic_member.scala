// A case class's inherited member is matched at the class site (dotty's `matchingMember`):
// `T[Int]`'s `productElement(i: A)` is `productElement(i: Int)`, the existing definition, while
// `U[String]`'s is another overload beside the one the rule gives.
trait T[A]:
  def productElement(i: A): Any = 99
case class C(x: Int) extends T[Int]

trait U[A]:
  def productElement(i: A): Any = 7
case class E(x: Int) extends U[String]

@main def run(): Unit =
  println((C(1): Product).productElement(0))
  println(C(1).productElement(0))
  println((E(5): Product).productElement(0))
  println(E(5).productElement("s"))
  println(E(5).productElement(0))
