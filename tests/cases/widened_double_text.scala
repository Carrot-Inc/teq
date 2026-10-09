//> using platform js
// An `Int`, `Short` or `Byte` value widened to a `Double` or a `Float` prints as one (`3.0`) on
// the JVM, whatever the receiver: a literal, a constant, an operation, a conditional, a match
// or a block, printed, concatenated, interpolated or boxed.
def one(): Int = 1
def ifAny(b: Boolean): Any = (if b then 1 else 2).toDouble
def matchAny(b: Boolean): Any = (b match { case true => 3; case _ => 4 }).toDouble
def blockAny(b: Boolean): Any = { val q = 1; q + 4 }.toDouble
def tryAny(s: String): Any = (try s.toInt catch { case _: NumberFormatException => -1 }).toDouble
object K:
  final val f = 7

@main def run(): Unit =
  var i = -2
  var b = true
  var l = 5L
  val s: Short = 3
  val by: Byte = 5
  println(3.toDouble)
  println((i + 1).toDouble)
  println((-32768: Short).toDouble)
  println((i + 1).toDouble.toDouble)
  println(K.f.toDouble)
  println((K.f * 2).toDouble)
  println(by.toDouble)
  println((i * 3).toDouble)
  println((i << 2).toDouble)
  println(-i.toDouble)
  println((-i).toDouble)
  println(i.toDouble)
  println((i + 1).toFloat)
  println(3.toFloat)
  println((i + 1).toLong)
  println('a'.toDouble)
  println((i + 1).toDouble + 0.5)
  println("" + (i + 1).toDouble)
  println(s"${(i + 1).toDouble}")
  val d: Double = i + 1
  println(d)
  println((if b then 1 else 2).toDouble)
  println((i match { case -2 => 7; case _ => 8 }).toDouble)
  println({ val y = i; y + 2 }.toDouble)
  println(one().toDouble)
  println((one() + i).toDouble)
  println((l + 1).toDouble)
  println((s + 1).toDouble)
  println((s * s).toFloat)
  println((i / 2).toDouble)
  println((i % 2).toDouble)
  println((~i).toDouble)
  println(List((i + 1).toDouble, 3.toDouble))
  val any: Any = (i + 1).toDouble
  println(any)
  println(Some(3.toDouble))
  println((3.toDouble, (i + 1).toFloat))
  println(math.max((i + 1).toDouble, -5.0))
  println((Int.MaxValue + 1).toDouble)
  println(ifAny(true))
  println(matchAny(true))
  println(blockAny(true))
  val thrown: Any = (if b then 1 else throw new RuntimeException()).toDouble
  println(thrown)
  val unmatched: Any = (b match { case true => 1; case _ => throw new RuntimeException() }).toDouble
  println(unmatched)
  println(tryAny("x"))
  println(tryAny("5"))
  println((try i + 1 finally ()).toDouble)
  println(((i + 1).toDouble: Any))
  println((3.toDouble: Any))
  println(List((i + 1).toDouble: Any))
