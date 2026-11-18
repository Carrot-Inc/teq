// Under Console.withOut, which of the stream's overloads the prelude's and Console's print and println reach, for the empty string,
// no argument, a text, a number, an object whose text is empty, and a null: `println("")` is the stream's println(Object), as
// `println(x)` of any other text, and `println()` alone its println(). The expected lines are scalac's on the JDK.
import java.io.*
class Sub(o: OutputStream, log: java.lang.StringBuilder) extends PrintStream(o, false):
  private def l(s: String): Unit = log.append(s + ";")
  override def println(): Unit = { l("println()"); super.println() }
  override def println(s: String): Unit = { l("println(String:" + s + ")"); super.println(s) }
  override def println(x: Any): Unit = { l("println(Object:" + x + ")"); super.println(x) }
  override def println(i: Int): Unit = { l("println(I)"); super.println(i) }
  override def print(s: String): Unit = { l("print(String:" + s + ")"); super.print(s) }
  override def print(x: Any): Unit = { l("print(Object:" + x + ")"); super.print(x) }
object Empty:
  override def toString = ""
object Probe:
  def main(args: Array[String]): Unit =
    val calls: List[(String, () => Unit)] = List(
      "println(\"\")" -> (() => println("")), "println()" -> (() => println()), "println(\"a\")" -> (() => println("a")),
      "println(1)" -> (() => println(1)), "println(Empty)" -> (() => println(Empty)), "println(null)" -> (() => println(null)),
      "print(\"\")" -> (() => print("")), "print(\"a\")" -> (() => print("a")), "print(Empty)" -> (() => print(Empty)),
      "scala.Console.println(\"\")" -> (() => scala.Console.println("")), "scala.Console.println()" -> (() => scala.Console.println()),
      "scala.Console.print(\"\")" -> (() => scala.Console.print("")))
    for (name, call) <- calls do
      val log = new java.lang.StringBuilder
      val b = new ByteArrayOutputStream
      scala.Console.withOut(new Sub(b, log)) { call() }
      println(name + " | " + log + " | " + b.toString("UTF-8").replace("\n", "\\n"))
