import scala.language.implicitConversions
class A { def foo = 1 }
class B { def foo = 2 }
implicit def ua(u: Unit): A = A()
implicit def ub(u: Unit): B = B()
def t(c: Boolean) = (if c then println()).foo
