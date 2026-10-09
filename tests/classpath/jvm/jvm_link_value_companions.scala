// jars: scala-library abi-callbacks-lib
// std: scala-library
// The companion of a value class holds scalac's instance copies of its extension methods,
// `Meters$.MODULE$.plus$extension(u, that)`, beside the static ones of the class.
import abi.ReflectValue

class Meters(val value: Double) extends AnyVal:
  def plus(that: Meters): Meters = Meters(value + that.value)
  def scaled(k: Int): Double = value * k
object Meters:
  def zero: Meters = Meters(0.0)
case class Label(text: String) extends AnyVal:
  def shout: String = text.toUpperCase

@main def run(): Unit =
  println(Meters(1.5).plus(Meters.zero).value)
  println(Label("a").shout)
  println(ReflectValue.extension("Meters", "scaled", 2.0, 3))
  println(ReflectValue.extension("Label", "shout", "hey"))
