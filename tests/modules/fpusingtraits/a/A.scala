package a2
trait I:
  def show(x: String): Unit
class A
trait B1(using I) extends A { summon[I].show("B1") }
trait B2(using I) extends B1 { summon[I].show("B2") }
trait C1 extends B1
trait C2 extends B2
class D(using I) extends A, C1
class E(using I) extends D(using new I { def show(x: String) = println(s"superD: $x")}), C2
class F(using I) extends A, C2
object Api { @annotation.nowarn("msg=duplicated at each inline site") inline def g(using I): A = new A with C2 {} }
