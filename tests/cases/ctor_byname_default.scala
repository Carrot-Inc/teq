// A by-name constructor parameter left to its default: the caller passes a thunk around the default getter's call,
// and the getter returns the value (the erasure of the parameter's result type), as a method's does; the thunk is run
// at each read of the parameter, so the default's effect is printed once a read. A primary constructor's, a second
// clause's reading the first, a generic one's, a secondary constructor's, an enum's in a class (its getter on the class
// itself), a local class's secondary constructor's (its getter read on the local companion, which the thunk captures),
// and a method's beside them; an explicit argument passes the given thunk.
class X(f: => String = { println("eval X"); "d" }) { def get: String = f + "," + f }
class Y(n: Int)(f: => String = { println("eval Y"); "e" + n }) { def get: String = f }
class W[A](a: A)(f: => A = { println("eval W"); a }) { def get: A = f }
class Z(g: String) {
  def this(n: Int, f: => String = { println("eval Z"); "z" }) = this(f + n + "," + f)
  def get: String = g
}
class Outer {
  enum E(f: => String = { println("eval E"); "en" }) {
    case A extends E()
    def get: String = f + "," + f
  }
  def use: String = E.A.get
}
object M { def m(f: => String = { println("eval M"); "m" }): String = f + "," + f }

@main def run(): Unit =
  println(new X().get)
  println(new X({ println("given"); "g" }).get)
  println(new Y(1)().get)
  println(new W("w")().get)
  println(new Z(2).get)
  println(new Outer().use)
  println(M.m())
  class L(g: String) {
    def this(n: Int, f: => String = { println("eval L"); "l" }) = this(f + n + "," + f)
    def get: String = g
  }
  println(new L(3).get)
