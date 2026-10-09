// Adapted from scala3 tests/run/protectedSuper.scala (Apache-2.0, see tests/scala3/README.md); replaced: `extends App` is a main method, the classes nested in classes and traits are left out.
package p {
  class A {
    protected def foo(): Int = 1
    protected def fuzz(): Int = 2
  }
}
package q {
  class B extends p.A {
    def bar() = foo()
  }
  trait Inner extends p.A {
    def bar() = foo()
    def baz() = fuzz()
  }
}
class C extends p.A with q.Inner

object Test {
  def main(args: Array[String]): Unit = {
    val b = new q.B
    assert(b.bar() == 1)
    val c = new C
    assert(c.bar() == 1)
    assert(c.baz() == 2)
    println("ok")
  }
}
