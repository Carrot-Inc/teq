// Adapted from scala3 tests/run/mixins.scala (tests 1 to 3) (Apache-2.0, see tests/scala3/README.md); replaced: `Console.println` is `println`, the semicolons are dropped.

// Test 1: "super" coming from mixins
object Test1 {
  class A {
    def f = "A::f"
  }

  class B extends A {
    override def f = "B::f"
  }

  trait M1 extends A {
    override def f = "M1::" + super.f
  }

  class C extends B with M1 {
    override def f = super[M1].f
  }

  def test(): Unit = {
    val c = new C
    println(c.f)
  }
}

// Test 2: qualified "super" inside of the host class
object Test2 {
  class M1 {
    def f = "M1::f"
  }

  trait M2 {
    def f = "M2::f"
  }

  trait M3 {
    def f = "M3::f"
  }

  class Host extends M1 with M2 with M3 {
    override def f = super[M1].f + " " + super[M2].f + " " + super[M3].f
  }

  def test(): Unit = {
    val h = new Host
    println(h.f)
  }
}

// Test 3: mixin evaluation order (bug 120)
object Test3 {
  class A(x: Unit, y: Unit) {
    println("A")
  }

  trait B {
    println("B")
  }

  class C extends A({ println("one") }, { println("two") })
          with B {
    println("C")
  }

  def test() = {
    val c = new C()
  }
}

object Test {
  def main(args: Array[String]): Unit = {
    Test1.test()
    Test2.test()
    Test3.test()
  }
}
