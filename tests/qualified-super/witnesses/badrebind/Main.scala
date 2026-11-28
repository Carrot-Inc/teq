class X
class Y extends X
trait A[+T] { def foo: T = null.asInstanceOf[T] }
trait B extends A[X] { override def foo: X = new X }
trait C extends A[Y] { override def foo: Y = new Y; class I { def f: Y = C.super.foo } }
class Fail extends B with C
@main def run(): Unit = { val f = new Fail; val y: Y = (new f.I).f; println(y) }
