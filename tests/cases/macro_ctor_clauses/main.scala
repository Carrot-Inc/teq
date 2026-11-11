class C(val a: Int)(val b: String)
class D[T](val a: T)(val b: Int)(using val c: String)
class F(val a: Int)(val b: Int)(val c: Int)
class G[A, B](val a: A)(val b: B)
class BN(x: => Int)(val y: Int):
  def x2: Int = x
object Main:
  def main(args: Array[String]): Unit =
    println(Shape.of(new C(1)("x")))
    println(Shape.of(new D[Int](1)(2)(using "u")))
    println(Shape.of(new D(1)(2)(using "u")))
    println(Shape.of(new C(a = 1)(b = "y")))
    println(Shape.of(new F(1)(2)(3)))
    println(Shape.of(new G(1)("g")))
    var n = 1
    println(Shape.of(new BN(n + 1)(2)))
    println(Shape.inner(new C(1)("x")))
    println(Shape.inner(new F(1)(2)(3)))
