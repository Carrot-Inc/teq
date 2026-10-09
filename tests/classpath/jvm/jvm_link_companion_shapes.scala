// jars: scala-library abi-callbacks-lib
// std: scala-library
// Companion shapes as scalac writes them: a user `copy` or `apply` of other parameters beside
// the synthesized ones, `unapply` answering a Boolean for a case class without parameters,
// `unapplySeq` for a repeated last parameter, the default getter of a second clause taking the
// first.
import abi.{Reflect, ReflectModule}
case class Copying(x: Int):
  def copy(x: Int): Copying = Copying(x + 100)
case class Empty()
case class Rep(a: Int, rest: String*)
case class Curried(x: Int)(val y: Int = x * 2)
case class Over(x: Int)
object Over:
  def apply(s: String): Over = Over(s.length)
@main def run(): Unit =
  println(Seq(Copying(1).copy(2), Empty(), Rep(1, "a"), Curried(3)(), Over("abc"), Over(4)))
  println(Reflect.copy(Copying(1), 5))
  println(Reflect.copyDefault(Copying(7), 1))
  println(ReflectModule.call("Empty", "unapply", Empty()))
  println(ReflectModule.call("Rep", "unapplySeq", Rep(2, "b")))
  println(ReflectModule.call("Curried", "$lessinit$greater$default$2", 5))
  println(ReflectModule.overloads("Over", "apply"))
  println(ReflectModule.overloads("Empty", "unapply"))
