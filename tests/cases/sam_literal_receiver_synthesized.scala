// The members a class gets without writing them (`copy`, `toString`, `hashCode`, `getClass`, `productPrefix`,
// `ordinal`), named without a qualifier inside a function literal converted to a Java interface, are the enclosing
// instance's, as in a method or in a Scala function.
import java.util.concurrent.Callable

case class Point(x: Int, y: Int):
  def viaCopy(): String = { val c: Callable[String] = () => copy(x = 9).toString; c.call() }
  def viaToString(): String = { val c: Callable[String] = () => toString; c.call() }
  def viaHashCode(): Boolean = { val c: Callable[Boolean] = () => hashCode == Point(x, y).hashCode; c.call() }
  def viaProductPrefix(): String = { val c: Callable[String] = () => productPrefix; c.call() }
  def viaGetClass(): String = { val c: Callable[String] = () => getClass.getSimpleName; c.call() }

enum Colour:
  case Red, Green
  def viaOrdinal(): Int = { val c: Callable[Int] = () => ordinal; c.call() }

class Plain:
  def viaToString(): Boolean = { val c: Callable[Boolean] = () => toString.startsWith("Plain@"); c.call() }
  def viaHashCode(): Boolean = { val c: Callable[Boolean] = () => hashCode == System.identityHashCode(this); c.call() }
  def inFunction(): Boolean = { val f: () => Boolean = () => toString.startsWith("Plain@"); f() }

@main def run(): Unit =
  val p = Point(1, 2)
  println(s"copy: ${p.viaCopy()}")
  println(s"toString of a case class: ${p.viaToString()}")
  println(s"hashCode of a case class: ${p.viaHashCode()}")
  println(s"productPrefix: ${p.viaProductPrefix()}")
  println(s"getClass: ${p.viaGetClass()}")
  println(s"ordinal: ${Colour.Green.viaOrdinal()}")
  val q = new Plain
  println(s"toString of a plain class: ${q.viaToString()}")
  println(s"hashCode of a plain class: ${q.viaHashCode()}")
  println(s"toString in a Scala function: ${q.inFunction()}")
