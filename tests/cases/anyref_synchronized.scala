// `synchronized` on any reference, `this` included and left out, gives its body's value
// (scala3's pos/i9775 and pos/nullary_poly).
class Counter:
  private var n = 0
  def next(): Int = this.synchronized {
    n += 1
    n
  }
  def peek(): Int = synchronized(n)
object Main:
  def main(args: Array[String]): Unit =
    val o = new AnyRef
    println(o.synchronized(2))
    val c = Counter()
    c.next()
    println(c.next())
    println(c.peek())
    println("lock".synchronized("str"))
