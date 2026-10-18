// A dynamic call by a Class value whose class's name is no identifier: each target's reflective proxies and its
// run-time classes name it alike (Scala.js's written name on JavaScript, the JVM's encoded one under teq interp and
// on the JVM), so Selectable.applyDynamic finds the method, as scalac's does.
import scala.reflect.Selectable.reflectiveSelectable
class `Arg-X`
class Receiver:
  def accept(x: `Arg-X`): String = "accepted"
@main def run(): Unit =
  val cls: Class[?] = classOf[`Arg-X`]
  println(reflectiveSelectable(new Receiver).applyDynamic("accept", cls)(new `Arg-X`))
