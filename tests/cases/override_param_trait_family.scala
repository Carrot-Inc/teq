// The family of a trait over a parameter class: mixed into a class, an object, an anonymous class, through a
// trait below it, into a subclass's parent, a case class and its copy, and with a trait's val in the class's place.
import scala.annotation.nowarn
class P { val v: String = "p"; def read: String = v }
class A(override val v: String) extends P
trait TR extends A
trait TR2 extends TR { def extra: String = "x" }
@nowarn class B extends A("a-arg") { override val v: String = "b-body" }
@nowarn class C(override val v: String) extends B
class D extends C("c-arg") with TR
object O extends C("o-arg") with TR
class D2 extends C("c2-arg") with TR2
class E extends D
case class CC(n: Int) extends C("cc-arg") with TR
trait T { val t: String = "t"; def readT: String = t }
class KA(override val t: String) extends T
trait TRK extends KA
@nowarn class KB extends KA("ka-arg") { override val t: String = "kb-body" }
@nowarn class KC(override val t: String) extends KB
class KD extends KC("kc-arg") with TRK
@main def run(): Unit =
  println("class " + new D().read)
  println("object " + O.read)
  println("anonymous " + (new C("anon-arg") with TR).read)
  println("trait-below " + new D2().read)
  println("subclass " + new E().read)
  println("case " + CC(1).read + " " + CC(1).copy(n = 2).read)
  println("trait-val " + new KD().readT)
