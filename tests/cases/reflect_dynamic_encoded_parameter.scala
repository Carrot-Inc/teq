// A dynamic call by a Class value whose class's name is
// no identifier: Selectable.applyDynamic finds the method
// in scalac's program, and is to find it in teq's too.
import scala.reflect.Selectable.reflectiveSelectable
class `Arg-X`
class Receiver:
  def accept(x: `Arg-X`): String = "accepted"
@main def run(): Unit =
  val cls: Class[?] = classOf[`Arg-X`]
  println(reflectiveSelectable(new Receiver).applyDynamic("accept", cls)(new `Arg-X`))
