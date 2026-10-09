// A null unboxed from an erased Java result is the primitive's zero on the JVM, as scalac's BoxesRunTime.unboxToX gives it;
// JavaScript carries the null and its number arithmetic reads it as 0 (a Long's BigInt arithmetic and the interpreter
// refuse it: the recorded Int-typed null), so the shapes here stay within what JavaScript prints alike
//> using scala 3.8.4
object Main:
  def main(args: Array[String]): Unit =
    val m = new java.util.HashMap[String, Int]()
    val x: Int = m.get("none")
    println(x + 1)
    val d = new java.util.HashMap[String, Double]()
    val z: Double = d.get("none")
    println(z + 0.5)
    val b = new java.util.HashMap[String, Boolean]()
    println(if b.get("none") then "t" else "f")
    m.put("one", 1)
    println(m.get("one") + 1)
