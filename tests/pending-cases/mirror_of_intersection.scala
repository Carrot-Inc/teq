// `summon[Mirror.ProductOf[CC[Int] & CC[String]]]`:
// scalac 3.8.4 synthesizes the product mirror of the
// intersection's case class (its `MirroredElemTypes`
// the field's type seen from the receiver).
import scala.deriving.Mirror
case class CC[A](a: A)
def t = summon[Mirror.ProductOf[CC[Int] & CC[String]]]
def s = summon[Mirror.ProductOf[CC[Int] & Serializable]]

@main def run(): Unit = println(s.fromProduct(Tuple1(3)))
