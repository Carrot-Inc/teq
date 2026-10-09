// jars: scala-library abi-callbacks-lib
// std: scala-library
// Static forwarders of an object's inherited members (a program trait's, a jar trait's), and none
// for a name the companion class inherits, as scalac's conflicting-names rule reads its members.
import abi.{Reflect, ReflectStatics}
trait Helper:
  def help: Int = 1
class Base:
  def other: Int = 1
class Comp extends Base
object Comp:
  def other: Int = 2
  def own: Int = 3
object Obj extends Helper with Ordering[Int]:
  def compare(a: Int, b: Int) = a - b
@main def run(): Unit =
  println(Reflect.static("Comp", "own"))
  println(ReflectStatics.count("Comp", "other"))
  println(Reflect.static("Obj", "help"))
  println(Reflect.static("Obj", "lt", 1, 2))
  println(Reflect.static("Obj", "compare", 5, 2))
