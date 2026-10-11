// A method takes the defaults of the members it overrides (`SymDenotation.hasDefaultParams`),
// its getter found on the receiver (`Applications.findDefaultGetter`): an override without a
// default of its own applies the overridden one's, an override that declares one its own,
// whatever the static type of the receiver.
object Inherited:
  def mark(s: String): Int = { print(s); 1 }
  class A { def f(a: Int = 1): Int = a }
  class B extends A { override def f(a: Int): Int = a * 10 }
  class C { def g(a: Int = mark("A"))(b: Int): Unit = () }
  class D extends C { override def g(a: Int)(b: Int): Unit = () }
  class E { def k(a: Int = mark("E")): Unit = () }
  class F extends E { override def k(a: Int = mark("F")): Unit = () }
  trait Logger { def log(msg: String, level: Int = 3): String }
  class Console extends Logger { def log(msg: String, level: Int): String = s"$msg@$level" }

  def main(args: Array[String]): Unit =
    println(new B().f())
    val a: A = new B
    println(a.f())
    new D().g()(mark("B")); println()
    val e: E = new F
    e.k(); println()
    val l: Logger = new Console
    println(l.log("x"))
