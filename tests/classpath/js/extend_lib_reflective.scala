// jars: extend-lib
// targets: js interp jvm
// A reflective call tells a scalac-built library class's overloads apart by the classes of
// their parameters, explicit and through the conversion of a structural call. Not under
// --std=scala-library on JavaScript, whose scala-library `Selectable` reflects as on the JVM.
import scala.reflect.Selectable.reflectiveSelectable
import extendlib.*

@main def run(): Unit =
  val o = reflectiveSelectable(new Overloads)
  println(o.applyDynamic("f", classOf[Int])(1))
  println(o.applyDynamic("f", classOf[String])("s"))
  val oi: { def f(x: Int): String } = new Overloads
  println(oi.f(2))
