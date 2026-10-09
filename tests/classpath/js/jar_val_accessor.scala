// jars: traitfields-lib
// targets: js jvm
// A jar trait's val that a class inherits for the program's abstract val, where another class's val implements a
// def and that val alike: on JavaScript the program reads the val through a method, so the jar trait's val, whose
// body the build emits, takes the method too (an earlier head read `new L().value()` of a property), and so does
// the jar's class overriding it, which no class of the program extends (the next head read its property).
import tfl.{ValBase, ValChild}

trait V:
  val value: Int
trait D:
  def value: Int
class C extends D with V:
  val value = 42
class L extends ValBase with V

@main def run(): Unit =
  println((new C: D).value)
  println((new C: V).value)
  println((new L: V).value)
  println((new L: ValBase).value)
  println(new L().value)
  println((new ValChild: ValBase).value)
  println(new ValChild().value)
