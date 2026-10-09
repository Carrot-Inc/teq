// `import v.given` from a value brings its Scala 2 implicit conversions too, as a given selector
// imports old-style implicits under scalac.
import scala.language.implicitConversions
class C:
  implicit def cv(x: Int): String = x.toString

@main def run(): Unit =
  val c = new C
  import c.given
  val s: String = 1
  println(s)
