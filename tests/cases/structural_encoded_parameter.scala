//> using platform jvm
// A structural call whose parameter's class has a name that is no identifier, and that class's name at run time:
// the one name the JVM gives it (dotty's javaClassName, fullName.mangledString) on every JVM-named target.
import scala.reflect.Selectable.reflectiveSelectable
class `Arg-X`
class Receiver:
  def accept(x: `Arg-X`): String = "accepted"
@main def run(): Unit =
  val r: { def accept(x: `Arg-X`): String } = new Receiver
  println(r.accept(new `Arg-X`))
  println(classOf[`Arg-X`].getName)
  println(new `Arg-X`().getClass.getName)
