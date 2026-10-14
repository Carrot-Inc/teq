// `math.max`, `min`, `abs`, `signum`, `floorDiv` and `floorMod` are scala-library's overloads of each primitive
// kind, as `java.lang.Math`'s are the JDK's: an argument widened to the wider kind, the result of that kind and
// no erased value to unbox (the review's `MathShape`).
object Main:
  def main(args: Array[String]): Unit =
    val n: Int = math.max(args.length, 1)
    println(n)
    println(math.max(1, 2L))
    println(math.min(3, 2.5))
    println(math.abs(-7) + 1)
    println(math.abs(Int.MinValue))
    println(math.signum(-2L))
    println(math.signum(0.5f) == 1.0f)
    println(math.floorDiv(-7, 2))
    println(math.floorMod(-7L, 2L))
    println(Math.max(4L, 3))
    println(Math.abs(-1.5f))
    val xs = List(3, 9, 4)
    println(xs.foldLeft(0)(math.max))
