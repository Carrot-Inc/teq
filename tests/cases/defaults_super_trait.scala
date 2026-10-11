// A getter selected on `super` inside a trait is the one after the trait in the linearisation of
// the class that mixes it in (`Applications.findDefaultGetter` on a `super` receiver), called
// through an accessor of its own that each such class implements (`SuperAccessors`,
// `ResolveSuper`): the trait's own override's default is not taken, stacked traits each take the
// next one's, a getter takes the earlier clauses' arguments, a target that inherits its default
// takes its parent's, and a lambda of the trait calls through the same accessor. A call that
// leaves the argument out on the class takes the trait's getter, which overrides the superclass's
// (the JVM's class forwards to it, as scalac's mixin forwarders do).
def mark(s: String): Int = { print(s); 1 }
class Base { def f(a: Int = mark("A")): Int = a }
trait Layer extends Base {
  override def f(a: Int = mark("L")): Int = super.f() + a
  def control(): Int = super.f(10)
}
class C extends Base with Layer
trait One extends Base { override def f(a: Int = mark("1")): Int = super.f() + a }
trait Two extends Base { override def f(a: Int = mark("2")): Int = super.f() + a }
class D extends Base with One with Two
class G { def g(x: Int)(y: Int = x * 2): Int = x + y }
trait H extends G { override def g(x: Int)(y: Int = x * 3): Int = super.g(x + 1)() * 10 + y }
class K extends G with H
class Base0 { def h(a: Int = mark("B")): Int = a }
class Base1 extends Base0 { override def h(a: Int): Int = a + 100 }
trait T extends Base0 {
  override def h(a: Int = mark("T")): Int = super.h()
  def later(): () => Int = () => super.h()
}
class E extends Base1 with T
@main def main(): Unit = {
  println(new C().f(5))
  println(new C().control())
  println(new C().f())
  println(new D().f(5))
  println(new K().g(1)(0))
  println(new E().h(1))
  println(new E().later()())
}
