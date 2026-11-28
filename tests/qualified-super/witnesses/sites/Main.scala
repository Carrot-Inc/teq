package sites
class A { def m(): String = "A" }
class Plain extends A {
 def local = { def f = super.m(); f }
 def lambda = (() => super.m())()
 def qualified = (() => Plain.super.m())()
 def byname = { def take(x: => String) = x; take(super.m()) }
 val field = super.m()
 lazy val lazyfield = super.m()
 def default(x: String = super.m()) = x
 def guard = 1 match { case x if super.m() == "A" => "A"; case _ => "bad" }
}
class B extends A {
 override def m() = "B"
 class Inner { def f = B.super.m(); def lambda = (() => B.super.m())(); class Deep { def f = B.super.m() } }
 class Other { def f = B.super.m() }
 object Obj { def f = B.super.m() }
 def local = { class Local { def f = B.super.m() }; (new Local).f }
 def anon = new A { override def m() = B.super.m() }.m()
}
class Sub extends B { override def m() = "Sub" }
class Sub0 extends B
object O extends A { class Inner { def f = O.super.m() } }
trait T extends A { class Inner { def f = T.super.m() } }
class C extends A with T
class V(val x: Int) extends AnyVal { def f(x: Any) = super.equals(x) }
@main def run(): Unit = {
 val p = new Plain; println(List(p.local,p.lambda,p.qualified,p.byname,p.field,p.lazyfield,p.default(),p.guard).mkString(","))
 val b = new B; val s = new Sub; val z = new Sub0; val c = new C
 val i = new b.Inner
 println(List(i.f,i.lambda,(new i.Deep).f,(new b.Other).f,b.Obj.f,b.local,b.anon,(new s.Inner).f,(new z.Inner).f,(new O.Inner).f,(new c.Inner).f).mkString(","))
}
