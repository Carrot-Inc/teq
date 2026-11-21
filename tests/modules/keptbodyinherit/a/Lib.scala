package kbi

class A(val v: Int)
class B(val v: Int)

// Bodies the library keeps: an overloaded name an object partly inherits, whose inherited alternative stays selected
// on the object; an extractor of two using clauses after the scrutinee, whose arguments the pickle holds in one
// sequence; a constructor's own empty second clause, applied as written.
trait Ops:
  def take(n: Int): String = "ops" + n
object O extends Ops:
  def take(s: String): String = "o" + s
class Holder extends Ops:
  override def take(n: Int): String = "holder" + n
  def viaO: String = O.take(1)
object E:
  def unapply(x: Int)(using a: A)(using b: B): Option[Int] = Some(x + a.v + b.v)
object A:
  given fallback: A = A(1000)
class C(val a: Int)():
  def show: String = "c" + a

object Lib:
  given A = A(10)
  given B = B(100)
  def inherited: String = O.take(1) + " " + O.take("w")
  def held: String = (new Holder).viaO
  def extracted: Int = 1 match { case E(n) => n }
  def empty: String = new C(1)().show
