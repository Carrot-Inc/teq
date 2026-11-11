import scala.language.implicitConversions
class A { def foo = 1 }
class B { def foo = 2 }
object C {
  implicit def cv(u: Unit): A = A()
  implicit def cv[T](u: Unit): B = B()
}
import C.*
def t(c: Boolean) = (if c then println()).foo
