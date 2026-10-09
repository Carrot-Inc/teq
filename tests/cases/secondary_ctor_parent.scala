// A parent called through its secondary constructor without arguments (`extends B`, `extends B()`, an anonymous
// `new B {}`, `AtomicReference`'s `this()`) runs that constructor.
import java.util.concurrent.atomic.AtomicReference

class B(val x: Int):
  def this() = this(7)
  def this(s: String) = this(s.length)
class D extends B()
class E extends B
class F extends B("abc")
final class Slot extends AtomicReference[String]

@main def main(): Unit =
  println(new D().x)
  println(new E().x)
  println(new F().x)
  println(new B {}.x)
  val s = new Slot
  println(s.get() == null)
  println(s.compareAndSet(null, "a"))
  println(s.get())
