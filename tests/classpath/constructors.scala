// jars: scala-library
// The secondary constructors of loaded classes are alternatives of the constructor.
import scala.collection.mutable.ArrayBuffer
object Main:
  def main(args: Array[String]): Unit =
    val sb = new StringBuilder("x")
    sb.append("y")
    println(sb.toString)
    val cause = new IllegalStateException("inner")
    val e = new RuntimeException("m", cause)
    val e2 = new RuntimeException(cause)
    println(e.getMessage + e2.getMessage)
    val buf = new ArrayBuffer[Int](16)
    buf += 1
    println(buf.length)
    val empty = new ArrayBuffer[String]()
    println(empty.isEmpty)
