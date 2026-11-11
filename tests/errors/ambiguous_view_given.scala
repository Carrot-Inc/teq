import scala.language.implicitConversions
class A { def foo = 1 }
class B { def foo = 2 }
class X
given xa: Conversion[X, A] = _ => A()
given xb: Conversion[X, B] = _ => B()
def t = X().foo
