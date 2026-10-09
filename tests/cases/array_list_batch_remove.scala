// `ArrayList.removeAll` and `retainAll` are the JDK's `batchRemove`: each element's membership
// tested in order while the survivors are moved down behind the read cursor (the list seen from
// `contains` shows the moves), and where `contains` throws, the elements not yet read follow those
// kept and the removals before stand. `removeIf` tests every element first.
import java.util.*

object Main:
  def main(args: Array[String]): Unit =
    val xs = new ArrayList[String]()
    for x <- Seq("a", "b", "c", "b", "d", "e") do xs.add(x)
    class Probe(throwAt: String, members: String*) extends AbstractCollection[String]:
      def size(): Int = members.length
      def iterator(): Iterator[String] =
        val l = new ArrayList[String]()
        members.foreach(l.add)
        l.iterator()
      override def contains(x: Any): Boolean =
        println(s"contains $x ${xs.size()} $xs")
        if x == throwAt then throw new IllegalStateException("at " + x)
        members.contains(x)
    try println(xs.removeAll(new Probe("d", "b")))
    catch case e: IllegalStateException => println("stopped " + e.getMessage)
    println(xs)
    println(xs.retainAll(new Probe("none", "a", "d")))
    println(xs)
    println(xs.removeAll(new Probe("none", "x")))
    println(xs)
    try xs.retainAll(null) catch case _: NullPointerException => println("null")
    val ys = new ArrayList[String]()
    for x <- Seq("p", "q", "r") do ys.add(x)
    try ys.removeIf(x => { println(s"test $x ${ys.size()}"); if x == "r" then throw new IllegalStateException(); x == "p" })
    catch case _: IllegalStateException => println("stopped")
    println(ys)
