import scala.language.implicitConversions
class A { def +++(i: Int) = 1 }
class B { def +++(i: Int) = 2 }
class X
implicit def xa(x: X): A = A()
implicit def xb(x: X): B = B()
def t = X() +++ 1
