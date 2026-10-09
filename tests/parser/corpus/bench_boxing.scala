// Boxing-heavy generic code: a generic array-backed buffer, a generic matrix product over a
// numeric type class, generic sums and extrema through Ordering, Option and tuple values in
// loops, all instantiated at Int and at Double. On the JVM every element crosses a boxing
// boundary; JavaScript has none. The Doubles are folded into the checksum as scaled Longs.
object BoxingBench:
  // Rounds, timed iterations and whether the per-iteration line is printed on stderr, when the
  // command line gives none; bench/runtime.sh compiles copies with its own values in this line,
  // since Scala.js gives main no arguments.
  val defaults = "120 1 0"

  val modulus = 1000000007L

  def mix(acc: Long, x: Long): Long = (acc * 31L + x) % modulus

  trait Num[A]:
    def zero: A
    def plus(a: A, b: A): A
    def times(a: A, b: A): A
    def fromInt(i: Int): A
    def toLong(a: A): Long
    def lt(a: A, b: A): Boolean

  given Num[Int] with
    def zero: Int = 0
    def plus(a: Int, b: Int): Int = a + b
    def times(a: Int, b: Int): Int = a * b
    def fromInt(i: Int): Int = i
    def toLong(a: Int): Long = a.toLong
    def lt(a: Int, b: Int): Boolean = a < b

  given Num[Double] with
    def zero: Double = 0.0
    def plus(a: Double, b: Double): Double = a + b
    def times(a: Double, b: Double): Double = a * b
    def fromInt(i: Int): Double = i.toDouble
    def toLong(a: Double): Long = (a * 16.0).toLong
    def lt(a: Double, b: Double): Boolean = a < b

  given Num[Long] with
    def zero: Long = 0L
    def plus(a: Long, b: Long): Long = a + b
    def times(a: Long, b: Long): Long = a * b
    def fromInt(i: Int): Long = i.toLong
    def toLong(a: Long): Long = a
    def lt(a: Long, b: Long): Boolean = a < b

  // A generic buffer over an array of Any, the shape of a hand-written growable collection.
  final class Buffer[A](capacity: Int):
    private val cells = new Array[Any](capacity)
    private var count = 0
    def size: Int = count
    def add(a: A): Unit =
      cells(count) = a
      count += 1
    def apply(i: Int): A = cells(i).asInstanceOf[A]
    def update(i: Int, a: A): Unit = cells(i) = a
    def foldLeft[B](z: B)(f: (B, A) => B): B =
      var acc = z
      var i = 0
      while i < count do
        acc = f(acc, apply(i))
        i += 1
      acc
    def map[B](f: A => B): Buffer[B] =
      val out = new Buffer[B](count)
      var i = 0
      while i < count do
        out.add(f(apply(i)))
        i += 1
      out

  def fill[A](n: Int, f: Int => A): Buffer[A] =
    val b = new Buffer[A](n)
    var i = 0
    while i < n do
      b.add(f(i))
      i += 1
    b

  def dot[A](xs: Buffer[A], ys: Buffer[A])(using num: Num[A]): A =
    var acc = num.zero
    var i = 0
    while i < xs.size do
      acc = num.plus(acc, num.times(xs(i), ys(i)))
      i += 1
    acc

  final class Matrix[A](val n: Int, val cells: Buffer[A]):
    def apply(r: Int, c: Int): A = cells(r * n + c)

  def multiply[A](a: Matrix[A], b: Matrix[A])(using num: Num[A]): Matrix[A] =
    val n = a.n
    val out = new Buffer[A](n * n)
    var r = 0
    while r < n do
      var c = 0
      while c < n do
        var acc = num.zero
        var k = 0
        while k < n do
          acc = num.plus(acc, num.times(a(r, k), b(k, c)))
          k += 1
        out.add(acc)
        c += 1
      r += 1
    new Matrix[A](n, out)

  def trace[A](m: Matrix[A])(using num: Num[A]): A =
    var acc = num.zero
    var i = 0
    while i < m.n do
      acc = num.plus(acc, m(i, i))
      i += 1
    acc

  // The generic kernels at one element type: buffers, a dot product, a matrix square and its
  // trace, sum and maximum over a List through the type class, then everything through toLong.
  def kernel[A](n: Int, seed: Int)(using num: Num[A]): Long =
    val xs = fill(n, i => num.fromInt((i * 7 + seed) % 13))
    val ys = fill(n, i => num.fromInt((i * 3 + 1) % 11))
    val d = dot(xs, ys)
    val side = 12
    val m = new Matrix[A](side, fill(side * side, i => num.fromInt((i + seed) % 5)))
    val squared = multiply(m, m)
    val t = trace(squared)
    val list = xs.map(x => num.plus(x, num.fromInt(1))).foldLeft(List.empty[A])((l, x) => x :: l)
    val total = list.foldLeft(num.zero)(num.plus)
    val largest = list.foldLeft(num.zero)((a, x) => if num.lt(a, x) then x else a)
    var acc = num.toLong(d) % modulus
    acc = mix(acc, num.toLong(t) % modulus)
    acc = mix(acc, num.toLong(total) % modulus)
    acc = mix(acc, num.toLong(largest))
    acc

  // Option[Int] and (Int, Double) values made and taken apart in loops: the allocation that
  // Scala's own collections pay per element.
  def wrappers(n: Int): Long =
    var acc = 0L
    var i = 0
    while i < n do
      val opt: Option[Int] = if i % 3 == 0 then None else Some(i)
      acc = mix(acc, opt.map(_ * 2).getOrElse(-1).toLong)
      val pair = (i, i.toDouble / 8.0)
      val (a, b) = pair
      acc = mix(acc, a.toLong + (b * 8.0).toLong)
      i += 1
    val pairs = List.tabulate(n)(i => (i, i.toDouble * 0.5))
    val folded = pairs.foldLeft(0.0)((s, p) => s + p._2 * p._1.toDouble)
    mix(acc, (folded * 2.0).toLong % modulus)

  // A generic sort through Ordering, over boxed Ints and Doubles.
  def sorting[A](n: Int, seed: Int)(using num: Num[A], ord: Ordering[A]): Long =
    val xs = List.tabulate(n)(i => num.fromInt((i * 31 + seed) % 97))
    val sorted = xs.sorted
    val top = sorted.reverse.take(5)
    mix(num.toLong(top.head), num.toLong(sorted.head) + num.toLong(top.foldLeft(num.zero)(num.plus)))

  def round(seed: Int): Long =
    var acc = kernel[Int](256, seed)
    acc = mix(acc, kernel[Double](256, seed))
    acc = mix(acc, kernel[Long](256, seed))
    acc = mix(acc, wrappers(256))
    acc = mix(acc, sorting[Int](128, seed))
    acc = mix(acc, sorting[Double](128, seed))
    acc

  def work(rounds: Int): Long =
    var acc = 0L
    var r = 0
    while r < rounds do
      acc = mix(acc, round(r * 5 + 1))
      r += 1
    acc

  def main(args: Array[String]): Unit =
    val d = defaults.split(" ")
    val n = (if args.length > 0 then args(0) else d(0)).toInt
    val iterations = (if args.length > 1 then args(1) else d(1)).toInt
    val report = (if args.length > 2 then args(2) else d(2)).toInt == 1
    val times = new Array[Long](iterations)
    var checksum = 0L
    var i = 0
    while i < iterations do
      val started = System.nanoTime()
      checksum = work(n)
      times(i) = System.nanoTime() - started
      i += 1
    if report then System.err.println(line("boxing", n, times))
    println("boxing checksum " + checksum)

  def line(name: String, n: Int, times: Array[Long]): String =
    var total = 0L
    var steady = 0L
    var i = 0
    while i < times.length do
      total += times(i)
      if i >= times.length / 2 then steady += times(i)
      i += 1
    "bench " + name + " n=" + n + " k=" + times.length + " first_ns=" + times(0) +
      " total_ns=" + total + " steady_ns=" + steady / (times.length - times.length / 2)
