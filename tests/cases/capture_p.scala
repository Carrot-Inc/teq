// A generator's pattern takes the element in a parameter named `p$0`, as a product mirror's
// constructor function takes the product, and both build the case class named as it is.
import scala.deriving.Mirror

case class `p$0`(a: Int, b: Int)

@main def main(): Unit =
  println(for (a, b) <- List((1, 2)) yield `p$0`(b, a))
  println(summon[Mirror.ProductOf[`p$0`]].fromProduct((1, 2)))
