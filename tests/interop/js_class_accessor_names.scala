// An inherited private member of a JS class is no half of the subclass's property, and a property whose JS name is
// no identifier is one property of its two halves across the classes (JSExportsGen collects a JS name's
// alternatives that are not private). The expectations are scalac 3.8.4's under Scala.js 1.22.0
// (`scala-cli run --js --js-version 1.22.0 --scala 3.8.4`).
import scala.scalajs.js
import scala.scalajs.js.annotation.JSName
class A extends js.Object:
  private def p_=(v: Int): Unit = ()
class B extends A:
  def p: Int = 1
def first(): Unit =
  val b = new B
  println(b.p)
  val obj = js.Dynamic.global.Object
  val desc = obj.getOwnPropertyDescriptor(obj.getPrototypeOf(b), "p")
  println(js.isUndefined(desc.set))

class C extends js.Object:
  @JSName("p-q") def x: Int = 1
class D extends C:
  @JSName("p-q") def x_=(v: Int): Unit = ()
def second(): Unit =
  println(js.isUndefined(new D().asInstanceOf[js.Dynamic].selectDynamic("p-q")))

@main def run(): Unit =
  first()
  second()
