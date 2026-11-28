// A call to an override uses the default getter as declared by its generic parent.
trait Bnd
trait X extends Bnd
object Arg extends X

class A[T] { def m(a: T)(b: Int = 1): Int = b }
class B extends A[X] { override def m(a: X)(b: Int): Int = b + 10 }

class A2[T <: Bnd] { def m(a: T)(b: Int = 2): Int = b }
class B2 extends A2[X] { override def m(a: X)(b: Int): Int = b + 20 }

class A3[T] { def m(a: T)(b: Int = 3): Int = b }
class B3 extends A3[Int] { override def m(a: Int)(b: Int): Int = b + 30 }

class A4[T] { def m(a: T)(b: Int = 4): Int = b }
class B4 extends A4[Array[Int]] { override def m(a: Array[Int])(b: Int): Int = b + 40 }

class A5[T](t: T) { def m(a: Int)(b: T = t): T = b }
class B5 extends A5[String]("five") { override def m(a: Int)(b: String): String = b + "!" }

@main def main(): Unit =
  println(new B().m(Arg)())
  println(new B2().m(Arg)())
  println(new B3().m(3)())
  println(new B4().m(Array(4))())
  println(new B5().m(5)())
