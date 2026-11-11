import scala.language.implicitConversions
class X
class A { def foo = 1 }
class B { def foo = 2 }
object C {
  implicit def cv(x: X): A = A()
  implicit def cv[T](x: X): B = B()
}
import C.*
def t = X().foo
