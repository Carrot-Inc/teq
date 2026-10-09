// jars: scala-library abi-callbacks-lib
// std: scala-library
// Module classes as scalac writes them: a private constructor and a final `MODULE$`, for a written
// object, a companion the backend writes, an enum's and a file's.
import abi.{ReflectModule, Reflect}
case class P(x: Int)
object Named:
  val self = Named
  def hi = "hi"
enum E:
  case A, B
@main def run(): Unit =
  println(ReflectModule.privateConstructor("P$"))
  println(ReflectModule.privateConstructor("Named$"))
  println(ReflectModule.privateConstructor("E$"))
  println(ReflectModule.privateConstructor("jvm_link_module_classes$package$"))
  println(Named.self eq Named)
  println(Reflect.finalModule("P") && Reflect.finalModule("Named"))
