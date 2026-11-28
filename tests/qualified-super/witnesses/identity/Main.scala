package identity
import scala.annotation.targetName
class A {
 def m(x: Int): String = "int"
 def m(x: String): String = "str"
 def g[X](x: X): X = x
 def c(x: Int)(y: String): String = y
 def b(x: => String): String = x + x
 def v(x: String*): String = x.mkString
 @targetName("named") def n(x: Int): String = "n"
}
class B extends A {
 class I { def f = (B.super.m(1),B.super.m("s"),B.super.g("g"),B.super.c(1)("c"),B.super.b("b"),B.super.v("v"),B.super.n(1)) }
 class J { def f = B.super.m(2) }
}
trait U extends A { override def m(x: Int) = "U" }
trait T extends A with U { class I { def f = T.super[U].m(1) } }
class C extends A with T
@main def run(): Unit = { val b = new B; println((new b.I).f); val c = new C; println((new c.I).f) }
