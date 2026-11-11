// `summon[Mirror.ProductOf[CC[Int] & CC[String]]]`: scalac 3.8.4 synthesizes the product mirror of the intersection's
// case class (its `MirroredElemTypes` the field's type seen from the receiver). teq reports "no given instance of
// type ProductOf[CC[Int] & CC[String]] was found" (master the same). The closure adversary of the pass on 97f0487ac
// (round 24).
import scala.deriving.Mirror
case class CC[A](a: A)
def t = summon[Mirror.ProductOf[CC[Int] & CC[String]]]
def s = summon[Mirror.ProductOf[CC[Int] & Serializable]]

@main def run(): Unit = println(s.fromProduct(Tuple1(3)))
