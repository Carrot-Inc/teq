// jars: scala-library
// std: scala-library
// targets: jvm
// The JVM target's linking: scala-library's bytecode as the JVM std.
// Arrays: a varargs literal, an Array literal, ArrayOps, ClassTag-taking factories and main's
// Array[String] are java.util.ArrayList under teq; scala-library's descriptors take JVM arrays.
object Main:
  def main(args: Array[String]): Unit =
    println(List(1, 2, 3))
    val a = Array(3, 1, 2)
    println(a.sorted.mkString(","))
    println(Array.fill(2)("x").toList)
    println(Array.tabulate(3)(i => i * i).toSeq)
    println(Vector(1, 2).toArray.length)
