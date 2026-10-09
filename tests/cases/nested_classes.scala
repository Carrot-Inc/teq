// Classes, case classes, implicit classes and traits nested in a class, from source: each
// instance holds its enclosing one (scalac's `Outer$Inner` with its outer reference).
class Outer(prefix: String):
  private final case class Access(n: Int, tag: String):
    def show: String = s"$prefix$n$tag"

  final class Counter(start: Int):
    private var count = start
    def next(): Int =
      count += 1
      count
    def label: String = Outer.this.prefix + count

  implicit class Shout(s: String):
    def shout: String = prefix + s.toUpperCase + "!"

  trait Named:
    def name: String
    def full: String = prefix + name

  final class Leaf(val name: String) extends Named

  class Level1(val x: Int):
    class Level2(val y: Int):
      def sum: String = prefix + (x + y)
    def pair(y: Int): String = Level2(y).sum

  def make(n: Int): String = Access(n, "m").show
  def matched(n: Int): String = Access(n, "p") match
    case Access(k, t) if k > 1 => s"big $k$t"
    case Access(k, _) => s"small $k"
  def copied: String = Access(1, "c").copy(n = 5).show
  def equalAccesses: Boolean = Access(2, "e") == Access(2, "e")
  def shouted: String = "hey".shout
  def leaf: String = Leaf("leaf").full
  def counted: String =
    val c = new Counter(10)
    c.next()
    c.label
  def nested: String = Level1(1).pair(2)

object Main:
  def main(args: Array[String]): Unit =
    run()
    more()

def run(): Unit =
  val o = Outer("#")
  println(o.make(3))
  println(o.matched(1))
  println(o.matched(4))
  println(o.copied)
  println(o.equalAccesses)
  println(o.shouted)
  println(o.leaf)
  println(o.counted)
  println(o.nested)
  println(Outer("*").nested)

trait Api:
  def base: String
  final case class Item(n: Int):
    def show: String = base + n
  def item(n: Int): String = Item(n).show

object ApiImpl extends Api:
  def base = "api:"

class ApiOf(b: String) extends Api:
  def base = b

def more(): Unit =
  println(ApiImpl.item(1))
  println(ApiOf("x").item(2))
