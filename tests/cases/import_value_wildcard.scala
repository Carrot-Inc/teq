// `import v.*` from a stable local value, a lambda's parameter or a local `val`: the value's
// members by name, a result seen from the value's path, and the value's conversions, called on
// it; a selector `x as _` leaves `x` out (scalac: "root 1", "out 2", "len 3", "root 4", "len 5").
import scala.language.implicitConversions

class Dsl(n: Int):
  type Out = String
  def root: String = s"root $n"
  def make: Out = s"out $n"
  def size(s: String): String = s"len ${s.length}"
  implicit def conv(i: Int): String = "x" * i
  def conv2: Int = n

object Build:
  def use[A](n: Int)(f: Dsl => A): A = f(new Dsl(n))

@main def main(): Unit =
  println(Build.use(1) { dsl => import dsl.*; root })
  println(Build.use(2) { dsl =>
    import dsl.*
    val o: dsl.Out = make
    o
  })
  println(Build.use(3) { dsl => import dsl.*; size(3) })
  val d = new Dsl(4)
  import d.*
  println(root)
  println(Build.use(5) { dsl => import dsl.{conv as _, *}; size("hello") })
