trait A:
  def f: String = "A"
  def g: String = "A.g"
  lazy val lz: String =
    println("  A.lz computed")
    "lzA"
trait B extends A:
  override def f: String = "B"
trait C extends A:
  override def f: String = "C"
trait D:
  def h: String = "D.h"
  def g: String = "D.g"

class X1 extends A with B
class X2 extends B with C
class X3 extends C with B
class X4 extends A with D:
  override def g: String = "X4.g"
class X6 extends B with C:
  override def f: String = "X6"
object O extends B with C

trait Named:
  def name: String
  def greet: String = "hi " + name
class Person(val name: String) extends Named
class Robot extends Named:
  val name = "R2"
class Lazy extends Named:
  lazy val name =
    println("  Lazy.name computed")
    "lazy"

@main def main(): Unit =
  println(X1().f)
  println(X2().f)
  println(X3().f)
  println(X4().g + " " + X4().h)
  println(X6().f)
  println(O.f)
  println(Person("p").greet)
  println(Robot().greet)
  val l = Lazy()
  println(l.greet + " " + l.greet)
  val x1 = X1()
  println(x1.lz + x1.lz)
  val as: List[A] = List(X1(), X2(), X3(), O)
  println(as.map(_.f))
