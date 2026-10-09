// `clone` overrides a Java method, so it is called with or without the parentheses.
import scala.collection.mutable.{ArrayBuffer, HashMap}
object Main:
  def main(args: Array[String]): Unit =
    val a = Array(1L, 2L)
    val b = a.clone
    b(0) = 9L
    println(a.mkString(",") + " " + b.mkString(","))
    val c = a.clone()
    c(1) = 7L
    println(c.mkString(","))
    val buf = ArrayBuffer(1, 2)
    val copy = buf.clone
    copy += 3
    println(buf.mkString(",") + " " + copy.mkString(","))
    val m = HashMap("a" -> 1)
    val m2 = m.clone
    m2("b") = 2
    println(m.size.toString + " " + m2.size)
