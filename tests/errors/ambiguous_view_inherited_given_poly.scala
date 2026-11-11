import scala.language.implicitConversions
trait L
trait R
class X extends L with R
class A { def foo = 1 }
class B { def foo = 2 }
class C[T](t: T) { given cv[U <: X]: Conversion[U, T] = _ => t }
object L extends C[A](A())
object R extends C[B](B())
def t = X().foo
