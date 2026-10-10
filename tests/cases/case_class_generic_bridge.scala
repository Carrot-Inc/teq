// `Supply[A]`'s `productElement(i: A)` is the existing definition of `Product`'s under
// `Supply[Int]` (dotty's `matchingMember` at the class site) and an overload beside the rule's
// under `Supply[String]`: a call through `Product` reaches the one the class has, through the
// bridge that answers `Product`'s name.
trait Supply[A]:
  def productElement(i: A): Any = 57
case class Exact(n: Int) extends Supply[Int]
case class Other(n: Int) extends Supply[String]

@main def run(): Unit =
  println((Exact(12): Product).productElement(0))
  println((Other(12): Product).productElement(0))
  println(Other(12).productElement("s"))
  println(Exact(12).productElement(0))
