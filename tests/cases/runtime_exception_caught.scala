// A program that catches by a base type alone sees the runtime's failures as the exceptions the JVM gives, the
// `java.util` one included: the classes the runtime throws are linked by a `try`, whichever it names.
object Main:
  def show(label: String)(body: => Any): Unit =
    try { body; println(label + ": no failure") }
    catch case e: Exception => println(label + ": " + e.getClass.getName + ": " + e.getMessage)
  def main(args: Array[String]): Unit =
    show("head")(Nil.head)
    show("get")(None.get)
    show("key")(Map.empty[Int, Int](1))
    show("index")(List(1)(5))
    show("next")(Iterator.empty[Int].next())
    show("tail")(Nil.tail)
    show("number")("ab".toInt)
    try throw new java.util.NoSuchElementException("made")
    catch case e: Throwable => println("throwable: " + e)
