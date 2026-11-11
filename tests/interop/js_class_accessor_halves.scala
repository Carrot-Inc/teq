// A JS class's property is its getter and setter together, an inherited half among them (Scala.js's JSExportsGen
// makes one property of a name's alternatives over the bases): a subclass's own setter does not hide the getter it
// inherits, nor its own getter the setter. The expectation is scalac 3.8.4's under Scala.js 1.22.0
// (`scala-cli run --js --js-version 1.22.0 --scala 3.8.4`).
import scala.scalajs.js
class Base extends js.Object:
  var seen = 0
  def x: Int = 1
  def x_=(v: Int): Unit = seen = v
class Child extends Base:
  override def x_=(v: Int): Unit = seen = v * 10
class Kid extends Base:
  override def x: Int = 5
@main def run(): Unit =
  val c = new Child().asInstanceOf[js.Dynamic]
  println(js.isUndefined(c.x))
  println(c.x)
  c.x = 3
  println(c.seen)
  val k = new Kid().asInstanceOf[js.Dynamic]
  println(k.x)
  k.x = 4
  println(k.seen)
