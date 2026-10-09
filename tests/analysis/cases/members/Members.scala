package mem

// Access, modifiers and definitions of every kind.
abstract class Base(val a: Int, var b: String, c: Double):
  protected def prot: Int = 1
  private[mem] def pkg: Int = 2
  protected[mem] def protPkg: Int = 3
  private def priv: Int = 4
  private[this] val local: Int = 5
  def abstr(x: Int): Int
  lazy val lz: Int = 6
  implicit def conv(s: String): Int = s.length
  def curried(a: Int)(b: Int)(using c: String): Int = a

class Impl(x: Int) extends Base(x, "b", 1.0):
  override def abstr(x: Int): Int = x
  override protected def prot: Int = 7
  def this() = this(0)
  def overloaded(i: Int): Int = i
  def overloaded(s: String): String = s
  class Inner(val n: Int)
  object Nested:
    def make: Inner = new Inner(1)

trait Stack:
  def push(x: Int): Unit
  def top: Int

trait Doubling extends Stack:
  abstract override def push(x: Int): Unit = super.push(x * 2)

given intOrd: Ordering[Int] = Ordering.Int

extension (s: String)
  def shout: String = s.toUpperCase

class Box[A](val a: A):
  def local: Int =
    class Local(val v: Int)
    new Local(1).v
  def anon: Runnable = new Runnable { def run(): Unit = () }
