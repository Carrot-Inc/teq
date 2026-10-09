// jars: scala-library
// std: scala-library
// scala-library's `StringContext.s` and `raw` have no bytecode (Scala 2 macros in its source):
// a call scalac cannot fold is `standardInterpolator` over the context's parts.
import scala.collection.mutable
@main def run(): Unit =
  val n = 3
  println(s"n=$n ${n + 1}")
  println(raw"a\nb $n")
  println(StringContext("a", "b").s(1))
  println(StringContext("x\\n", "").raw(2))
  val sc = StringContext("<", ">")
  println(sc.s("mid"))
  val b = new mutable.ArrayBuffer[Int](8)
  b += 1
  println(b)
  val sb = new mutable.StringBuilder("ab")
  sb.append(1)
  println(sb.toString)
  println(new scala.util.Random(42).nextInt(10) >= 0)
  val q = mutable.Queue(1, 2)
  println(q.dequeue())
  println(new Some(3).get + new scala.collection.immutable.Range.Inclusive(1, 3, 1).sum)
  println(scala.math.BigDecimal("1.50").setScale(1, scala.math.BigDecimal.RoundingMode.HALF_UP))
