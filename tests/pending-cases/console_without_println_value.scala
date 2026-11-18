// Under Console.withOut the reference's println(x) is the stream's println of the VALUE: a subclass's
// println(Object) sees an Integer for println(1) and a String for println(""). teq carried the call with the text
// the prelude rendered (a String for println(1), println("") as println()) until the prelude's println(x) handed
// the stream the value itself.
import java.io.*
class Spy(b: ByteArrayOutputStream) extends PrintStream(b):
  override def println(x: Any): Unit = super.print("O[" + (if x == null then "null" else x.getClass.getSimpleName) + "]")
  override def println(): Unit = super.print("N[]")
object Main:
  def main(args: Array[String]): Unit =
    val b = new ByteArrayOutputStream
    scala.Console.withOut(new Spy(b)) { println(1); println(""); println("x") }
    println(b.toString("UTF-8"))
