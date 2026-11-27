// The no-defaults preference among equally specific alternatives (Applications.scala 2756, hasDefaultParams over the
// overridden members): an override whose overridden member declares a default is an alternative with defaults, so of
// two alternatives that both apply to every argument given the one with none wins, whichever is declared first and
// whether the default is a class's, a trait's or two levels up. scalac 3.8.4 prints 27, 17 and 19.
object Pref {
  trait X; trait Y; object Z extends X with Y
  class A { def m(a: X, b: Int = 1): Int = b }
  class B extends A { override def m(a: X, b: Int): Int = b + 10; def m(a: Y, b: Int): Int = b + 20 }
  trait TQ { def m(a: Y, b: Int = 2): Int }
  class C extends TQ { def m(a: X, b: Int): Int = b + 10; def m(a: Y, b: Int): Int = b + 20 }
  class G0 { def m(a: Y, b: Int = 2): Int = b }
  class G1 extends G0 { override def m(a: Y, b: Int): Int = b + 5 }
  class D extends G1 { def m(a: X, b: Int): Int = b + 12; override def m(a: Y, b: Int): Int = b + 20 }
}
@main def run(): Unit =
  println(new Pref.B().m(Pref.Z, 7))
  println(new Pref.C().m(Pref.Z, 7))
  println(new Pref.D().m(Pref.Z, 7))
