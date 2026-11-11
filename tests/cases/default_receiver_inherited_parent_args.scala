// A call with a default argument on an object whose superclass takes no arguments but extends one that does,
// through its primary constructor, a trait's parameter or a secondary constructor: touching the object runs the
// ancestor's constructor argument, so the call's first argument, lifted into a temporary before the receiver
// (EtaExpansion.liftArgs, the receiver an idempotent path), runs first.
class Base(x: Int)
class Mid extends Base({ print("I"); 0 })
object Obj extends Mid:
  def f(a: Int)(b: Int = 0): Unit = println("!")

trait Named(val name: String)
class Holder extends Named({ print("N"); "h" })
object Held extends Holder:
  def g(a: Int)(b: Int = 1): Unit = println(name)

class Plain:
  def this(x: Int) = { this(); print("S") }
class Via extends Plain({ print("I"); 0 })
object Second extends Via:
  def k(a: Int)(b: Int = 2): Unit = println("?")

@main def main(): Unit =
  Obj.f({ print("A"); 0 })()
  Held.g({ print("B"); 0 })()
  Second.k({ print("C"); 0 })()
