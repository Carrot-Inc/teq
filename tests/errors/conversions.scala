// expect: both method a2b in object Both and method a2b2 in object Both provide an extension method `b` on A
// expect: value c is not a member of A
// expect: type mismatch: found A, required C
// expect: value d is not a member of A

import scala.language.implicitConversions
class A
class B { def b: String = "b" }
class C { def c: String = "c" }
class D { def d: String = "d" }
object D { implicit def fromA(a: A): D = new D }
object Both {
  implicit def a2b(a: A): B = new B
  implicit def a2b2(a: A): B = new B
}
object Chain {
  implicit def a2b(a: A): B = new B
  implicit def b2c(b: B): C = new C
}
object Test {
  def f(): Unit = {
    import Both._
    println((new A).b)
  }
  def g(): Unit = {
    import Chain._
    println((new A).c)
    val c: C = new A
  }
  def h(): Unit = {
    println((new A).d)
    val d: D = new A
    println(d.d)
  }
}
