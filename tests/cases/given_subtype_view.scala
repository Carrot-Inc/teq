// A given of a type deriving from `<:<` is a view (`<:<` extends `Conversion` in scalac's library),
// and so is a using parameter whose type is bounded by one.
import scala.language.implicitConversions
class A
class B { def n = 42 }
given ev: (A <:< B) = (summon[A =:= A]).asInstanceOf[A <:< B]
def test(a: A): B = a
def bounded[Ev <: (A <:< B)](a: A)(using ev: Ev): B = a
@main def run(): Unit = println("compiled")
