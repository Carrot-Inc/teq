// What else holds a list or copies a prototype: an enum that takes a val as its own parameter, an enum's class
// case that does, an enum that mixes in a trait over the parameter's class; a class and an object whose only
// route to a parameter class is a trait (their superclass is the trait's), a body under it, an anonymous class.
import scala.annotation.nowarn
trait T { val t: String = "trait"; def readT: String = t }
// An enum that takes a trait's val as its own parameter: its cases are instances of it.
enum E(override val t: String) extends T { case One extends E("one"); case Two extends E("two") }
// An enum whose class case takes the val as a parameter, beside a case that does not.
enum F extends T { case Plain; case Par(override val t: String) }
class P { val v: String = "p"; def read: String = v }
// An enum over a class's val, and one that mixes in a trait that extends the class.
enum G(override val v: String) extends P { case A extends G("ga"); case B extends G("gb") }
trait TG extends P
enum H(override val v: String) extends P with TG { case A extends H("ha") }
// A class and an object whose only route to the parameter class is a trait: their superclass is the trait's.
class AD(override val v: String = "ad-default") extends P
trait TRD extends AD
class ED extends TRD
object OD extends TRD
@nowarn class BD extends AD("ad-arg") { override val v: String = "bd-body" }
trait TRB extends BD
class EB extends TRB
@nowarn class CD(override val v: String) extends BD
trait TRC extends CD
@main def run(): Unit =
  println("enum-own-param " + E.One.readT + " " + E.Two.readT + " " + E.One.t)
  println("enum-class-case " + F.Plain.readT + " " + F.Par("par").readT)
  println("enum-over-class " + G.A.read + " " + G.B.read)
  println("enum-with-trait " + H.A.read)
  println("trait-route " + new ED().read + " " + OD.read + " " + new EB().read)
  println("trait-route-anonymous " + (new CD("cd-arg") with TRC).read + " " + new TRD {}.read)
