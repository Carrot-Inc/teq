package reviewgen
class Box[A]
class Outer:
  class Ann[A] extends scala.annotation.StaticAnnotation
  def inner: Box[Int @Ann[String]] = ???
object GenericAnn:
  val outer: Outer = new Outer
  def qualified: Box[Int @outer.Ann[String]] = ???
  val a = outer.inner
  val b = qualified
