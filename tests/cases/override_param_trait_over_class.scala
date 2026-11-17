// A trait that extends a class which takes a val as a constructor parameter, mixed into a class below a body's
// redefinition and a parameter below that: the instance reads its parameter (a trait has no list of its own).
import scala.annotation.nowarn
class P { val v: String = "p"; def read: String = v }
class A(override val v: String) extends P
trait TR extends A
@nowarn class B extends A("a-arg") { override val v: String = "b-body" }
@nowarn class C(override val v: String) extends B
class D extends C("c-arg") with TR
@main def run(): Unit =
  val d = new D()
  println(d.read + "/" + d.v + " " + new C("c").read)
