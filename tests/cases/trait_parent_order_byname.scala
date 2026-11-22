// A class parent's by-name using parameter beside named arguments written out of their order (a trait's parameter cannot
// be by-name): the named arguments are evaluated in the order written, the by-name one where it is read, as scalac does.
class A(val s: String)
class B(val s: String)
class C(val s: String)
class T(x: Int)(using a: => A)(using b: B, c: C):
  def show = x.toString + " " + a.s + " " + a.s + " " + b.s + " " + c.s
class Sub extends T({ println("x"); 1 })(using { println("a"); A("wa") })(using c = { println("c"); C("wc") }, b = { println("b"); B("wb") })
object Main:
  def main(args: Array[String]): Unit =
    println(new Sub().show)
