// A trait's method implementing another trait's method that declares a default, in two classes that mix
// it under two such traits: the call through each trait takes that trait's default (the getter selected
// on the receiver).
trait T1 { def f(a: Int = 3): Int }
trait T2 { def f(a: Int = 7): Int }
trait O { def f(a: Int): Int = a + 1 }
class C1 extends T1 with O
class C2 extends T2 with O
enum E extends T1 with O { case A }

@main def run(): Unit =
  println((C1(): T1).f())
  println((C2(): T2).f())
  println((E.A: T1).f())
  println(C1().f(10))
