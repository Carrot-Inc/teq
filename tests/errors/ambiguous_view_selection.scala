import scala.language.implicitConversions
class A { def foo = 1 }
class B { def foo = 2 }
class X
implicit def xa(x: X): A = A()
implicit def xb(x: X): B = B()
def t = X().foo
