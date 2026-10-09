// An inline override's retained body, the classes an inline body makes and its reads of private
// members, which scalac reads through the accessors teq's pickle gives the class.
trait Fmt:
  def show(x: Int): String

class Counter(start: Int):
  private var count = start
  private def bump(n: Int): Int = { count += n; count }
  inline def tick(n: Int): Int = bump(n) + count

object Loud extends Fmt:
  override inline def show(x: Int): String = "int " + (x + 1)

object Makers:
  inline def runner(x: Int): Runnable = new Runnable { def run() = println("run " + x) }
  inline def boxed(x: Int): Int = { class Box(val v: Int); new Box(x * 3).v }

object InlineClassesAccess:
  def main(args: Array[String]): Unit =
    val c = new Counter(1)
    println(c.tick(2))
    val f: Fmt = Loud
    println(f.show(1) + " " + Loud.show(2))
    Makers.runner(5).run()
    println(Makers.boxed(4))
