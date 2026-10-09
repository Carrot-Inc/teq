// A boxed number widened to a wider type unboxes as its own kind on the JVM, where scalac's
// `BoxesRunTime.unboxToX` refuses another kind's box: every widening between two kinds of box has
// a node of its own (`ByteToInt`, `ShortToInt`, `IntToDouble` beside `IntToLong`), so `bytes.head`
// widened to an `Int` is unboxed as the `Byte` it is, and a literal widened once a parameter is
// solved is boxed as the parameter's kind. Whole doubles are printed as ints, since JavaScript
// prints them without the `.0`.
object P:
  val bs: List[Byte] = List(1, 2)
  val ss: List[Short] = List(3, 4)
  val is: List[Int] = List(5, 6)
  val cs: List[Char] = List('a')
  val p: (Int, Double) = (7, 1.5)
  def first[T](xs: List[T]): T = xs.head

@main def run(): Unit =
  val i: Int = P.bs.head
  val j: Int = P.ss.head
  val l: Long = P.bs.head
  val d: Double = P.ss.head
  val d2: Double = P.is.head
  val f: Float = P.bs.head
  val d3: Double = P.cs.head
  val s: Short = P.bs.head
  println(i + j + l)
  println(((d + d2 + f + d3) * 10).toInt)
  println(P.bs.head.toInt + P.ss.head.toInt + P.bs.head + 1 + s)
  println(P.ss.head.toDouble / 2)
  println(P.p._1.toDouble + P.p._2)
  println(P.first(P.bs) + 1.5)
  println(P.first(P.is).toDouble / 4)
  val folded = List((1, 0.5), (2, 1.5)).foldLeft(0.0)((acc, q) => acc + q._2 * q._1.toDouble)
  println(folded + 0.25)
  val any: Any = P.is.head.toDouble
  println(any == 5.0)
  val doubles: List[Double] = List(P.bs.head, 2, P.is.head)
  println(doubles.sum + 0.5)
  // A literal widened once the parameter is solved (`Math.min(total % 5L, 1)` takes the `Long`
  // overload) is boxed as the parameter's kind, which the result's unboxing expects.
  val total = 23L
  val m: Long = Math.min(total % 5L, 1)
  println(m + 1L)
  println(Math.max(5, 3.5) + 0.25)
