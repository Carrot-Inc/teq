// A lambda whose parameters are untupled from a tuple that holds a value class reads the element as the box it
// is in the tuple (the JVM once unboxed it as the underlying primitive and threw ClassCastException).
final case class Code(value: Int) extends AnyVal
final case class Name(value: String) extends AnyVal
final case class W[A](a: A) extends AnyVal

object Main:
  def main(args: Array[String]): Unit =
    val xs = List((Code(1), "a"), (Code(2), "b"))
    println(xs.map((code, text) => code.value))
    println(xs.map((code, text) => text))
    println(xs.map((code, text) => code))
    println(Map(Code(3) -> "c").map((code, text) => code.value))
    val f: ((Code, String)) => Int = (code, text) => code.value
    println(f((Code(4), "d")))
    println(List((Name("n"), 1)).map((name, i) => name.value))
    println(List(("a", Code(5))).map((text, code) => code.value))
    println(List((Code(6), "a", 1)).map((c, t, i) => c.value + i))
    println(List((W(1), W("s"))).map((a, b) => a.a + b.a.length))
    var n = 0
    List((Code(7), "a")).foreach((c, s) => n += c.value)
    Map(Code(8) -> "b").foreach((c, s) => n += c.value)
    println(n)
    val p = (Code(9), Name("m"), 3)
    println(p.reverse._3.value + p.init._2.value)
