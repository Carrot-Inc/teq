// A trait's overloaded alternative implementing another trait's method in the class that mixes both, and
// in a subclass two levels down: the call through the method's trait reaches the alternative.
trait T { def f(x: Int): Int }
trait O { def f(x: Int): Int = 2; def f(x: String): Int = 3 }
class C extends T with O
class C1 extends C
class C2 extends C1

@main def run(): Unit =
  val c = C()
  println("C " + c.f(1) + " " + c.f("s"))
  println("T " + (c: T).f(1))
  println("O " + (c: O).f(1))
  println("two down " + (C2(): T).f(1))
