// A refined member's info where it does not conform to the class member's is the two met
// (`Types.findMember`'s `goRefined`, `Denotations.meet`): `A1 & Z1`; two parts' equal refinements
// join (`goAnd`); a refinement's type parameters may have wider bounds (`matchingPolyParams`).
trait A1 { def a = "a" }
trait Z1 { def z = "z" }
trait Base { def f: A1; def m(x: Int): A1; val v: A1 }
// A refined result that does not conform to the class member's: the two met, `A1 & Z1`.
def g(b: Base { def f: Z1; def m(x: Int): Z1; val v: Z1 }): String =
  val x: A1 & Z1 = b.f
  val y: A1 & Z1 = b.m(1)
  val w: A1 & Z1 = b.v
  x.a + x.z + y.z + w.z
// Two separately declared equal refinements join.
def h(b: Base { def m(x: Int): Z1 } & Base { def m(x: Int): Z1 }): String = b.m(2).z
class AZ extends A1 with Z1
class C extends Base { def f: AZ = AZ(); def m(x: Int): AZ = AZ(); val v: AZ = AZ() }
trait P { def f[T <: AnyRef](t: List[T]): Any }
class Q extends P { def f[T <: AnyRef](t: List[T]): T = t.head }
def use(p: P { def f[T](t: List[T]): T }): String = p.f[String](List("poly"))
// A type parameter's bound that erases alike refines the member (`A1 & Z1` erases as `A1`).
trait G { def f[T <: A1 & Z1](x: T): Any }
class GI extends G { def f[T <: A1 & Z1](x: T): String = "erasure" }
def same(g: G { def f[T <: A1](x: T): String }): String = g.f(AZ())
// Two generic refinements of one signature join (`infoMeet`'s `PolyType`: the binders read as
// one, the bounds met), and a generic parent's result meets a refinement's.
trait GB { def f[T](x: T): Any }
class GS extends GB { def f[T](x: T): String = "joined" }
def joined(x: GB { def f[T](x: T): String } & GB { def f[U](x: U): String }): String = x.f[Int](1)
trait GP { def f[T](x: T): A1 }
class GQ extends GP { def f[T](x: T): AZ = AZ() }
def parentMet(x: GP { def f[U](x: U): Z1 }): A1 & Z1 = x.f[Int](1)
@main def run(): Unit =
  println(g(new C))
  println(h(new C))
  println(use((new Q).asInstanceOf[P { def f[T](t: List[T]): T }]))
  println(same((new GI).asInstanceOf[G { def f[T <: A1](x: T): String }]))
  println(joined((new GS).asInstanceOf[GB { def f[T](x: T): String } & GB { def f[U](x: U): String }]))
  println(parentMet((new GQ).asInstanceOf[GP { def f[U](x: U): Z1 }]).z)
