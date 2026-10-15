//> using platform jvm
// split-watch: jvm
import scala.deriving.Mirror

// A case class whose companion declares `Mirror.Product` itself: scalac's `SyntheticMembers`
// adds its own `Mirror.Product` parent beside the declared one, and the class file implements
// the interface once; a companion that declares none implements it once too.
case class D(x: Int)

object D extends Mirror.Product:
  type MirroredMonoType = D
  def fromProduct(p: Product): D = new D(p.productElement(0).asInstanceOf[Int])

case class E(y: Int)

object E

@main def run(): Unit =
  def mirrors(c: Class[?]): Int = c.getInterfaces.count(_ == classOf[Mirror.Product])
  println(mirrors(D.getClass))
  println(mirrors(E.getClass))
  println(summon[Mirror.ProductOf[D]].fromProduct(Tuple1(3)))
  println(summon[Mirror.ProductOf[E]].fromProduct(Tuple1(4)))
