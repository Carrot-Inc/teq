// A program of the Scala.js layer that never looks a class up by name: the class that carries
// @EnableReflectiveInstantiation is not named, so neither it nor the registry is written.
import scala.scalajs.js
import scala.scalajs.reflect.annotation.EnableReflectiveInstantiation

@EnableReflectiveInstantiation
class Unnamed(val n: Int)

@main def main(): Unit =
  println(js.typeOf(js.Dynamic.literal(a = 1)))
