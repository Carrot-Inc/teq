// After Scala 3's tests/pos/open-selftype.scala and tests/pos/i9352.scala: self types of
// traits, `this: T =>`, `self: T with U =>`, and a self type over a class.
trait Logger:
  def log(s: String): String = s"[$s]"
  def prefix: String = ">"

trait Named:
  def name: String

class Store(val items: List[Int]):
  def total: Int = items.sum

trait Service:
  self: Logger & Named =>
  def run: String = log(s"running $name")
  def viaThis: String = this.prefix + self.name
  def lambda: () => String = () => log(name)

trait Audited:
  this: Store =>
  def audit: String = s"total ${this.total} of ${items.size}"

trait Ordered2[A]:
  self: Ordering[A] =>
  def max2(a: A, b: A): A = if compare(a, b) >= 0 then a else b

class Good extends Service with Logger with Named:
  def name = "good"

class Shop(items: List[Int]) extends Store(items) with Audited

object IntOrd extends Ordering[Int] with Ordered2[Int]:
  def compare(a: Int, b: Int): Int = a - b

trait Base:
  self =>
  def base: Int = 1
  def self2: Base = self

trait Chained:
  self: Service with Logger with Named =>
  def both: String = run + viaThis

class All extends Chained with Service with Logger with Named:
  def name = "all"

@main def m(): Unit =
  val g = new Good
  println(g.run)
  println(g.viaThis)
  println(g.lambda())
  println(new Shop(List(1, 2, 3)).audit)
  println(IntOrd.max2(3, 5))
  val anon = new Service with Logger with Named { def name = "anon" }
  println(anon.run)
  println(new Base {}.self2.base)
  println(new All().both)
  val s: Service = g
  println(s.run)
