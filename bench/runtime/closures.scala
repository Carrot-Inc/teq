// Closures and higher-order functions: pipelines composed with andThen and compose, partial
// application of curried and placeholder forms, closures over mutable locals, iterators built
// lazily and consumed in a loop, by-name arguments, and lambdas passed through the collection
// methods. Only Longs and Ints reach the checksum.
object ClosuresBench:
  // Rounds, timed iterations and whether the per-iteration line is printed on stderr, when the
  // command line gives none; bench/runtime.sh compiles copies with its own values in this line,
  // since Scala.js gives main no arguments.
  val defaults = "100 1 0"

  val modulus = 1000000007L

  def mix(acc: Long, x: Long): Long = (acc * 31L + x) % modulus

  val inc: Long => Long = _ + 1L
  val dbl: Long => Long = x => (x * 2L) % modulus
  val sq: Long => Long = x => (x * x) % modulus
  def times(k: Long): Long => Long = x => (x * k) % modulus

  // A pipeline reduced from a list of functions, and a fixed composition, applied per element.
  def composePass(count: Int, seed: Int): Long =
    val steps: List[Long => Long] = List(inc, dbl, times((seed % 7 + 2).toLong), sq, inc)
    val f = steps.reduce(_ andThen _)
    val g = sq compose dbl compose inc
    var acc = 0L
    var i = 0
    while i < count do
      acc = mix(acc, f(i.toLong) + g(i.toLong))
      i += 1
    acc

  def clamp(lo: Long, hi: Long, x: Long): Long = if x < lo then lo else if x > hi then hi else x
  def scale(k: Long)(x: Long): Long = (k * x) % modulus

  // Partial application: a placeholder, a curried def with its first list applied, a method value.
  def partialPass(count: Int): Long =
    val clamp10 = clamp(10L, 1000L, _)
    val scale3 = scale(3L)
    val step: Long => Long = clamp10 andThen scale3
    val plusOne: Long => Long = inc
    var acc = 0L
    var i = 0
    while i < count do
      acc = mix(acc, step(i.toLong) + plusOne(i.toLong))
      i += 1
    acc

  def makeCounter(): () => Int =
    var c = 0
    () =>
      c += 1
      c

  // Closures over mutable locals: a recorder that updates two vars, counters with private state.
  def counterPass(count: Int): Long =
    var hits = 0
    var total = 0L
    val record = (x: Long) =>
      hits += 1
      total += x
    var i = 0
    while i < count do
      if (i & 3) != 0 then record(i.toLong)
      i += 1
    val c1 = makeCounter()
    val c2 = makeCounter()
    var j = 0
    while j < count do
      c1()
      if j % 2 == 0 then c2()
      j += 1
    mix(mix(hits.toLong, total), c1().toLong * 1000L + c2().toLong)

  // Iterators: a lazy chain from Iterator.from consumed with hasNext/next, a zip of two
  // iterators over a list, and Iterator.tabulate folded.
  def iteratorPass(count: Int): Long =
    val it = Iterator.from(1).map(x => x.toLong * 3L).filter(x => x % 2L == 1L).take(count)
    var acc = 0L
    while it.hasNext do
      acc = mix(acc, it.next())
    val xs = List.tabulate(count)(i => (i * 13) % 97)
    val zipped = xs.iterator.zip(xs.reverseIterator)
    while zipped.hasNext do
      val (a, b) = zipped.next()
      acc = mix(acc, (a * b).toLong)
    acc = mix(acc, Iterator.tabulate(count)(i => i.toLong * 2L).foldLeft(0L)(_ + _))
    acc

  def repeat(n: Int)(body: => Unit): Unit =
    var i = 0
    while i < n do
      body
      i += 1

  def orElse(a: Option[Long])(b: => Long): Long = a match
    case Some(x) => x
    case None => b

  def check(cond: => Boolean, onFail: => Long): Long = if cond then 0L else onFail

  // By-name arguments: a loop body, a fallback that is evaluated only on None, a lazy condition.
  def bynamePass(count: Int): Long =
    var acc = 0L
    var i = 0
    repeat(count) {
      acc = mix(acc, i.toLong)
      i += 1
    }
    var misses = 0L
    var k = 0
    while k < count do
      val opt = if (k & 1) == 0 then Some(k.toLong) else None
      acc = mix(acc, orElse(opt) {
        misses += 1
        k.toLong * 2L
      })
      acc = mix(acc, check(k % 3 != 0, {
        misses += 1
        7L
      }))
      k += 1
    mix(acc, misses)

  // Lambdas through the collection methods: filter/map/foldLeft capturing a local, collect with
  // a partial function literal, a list of functions folded over each element, sortBy, exists.
  def collectionPass(count: Int, seed: Int): Long =
    val base = seed % 11
    val xs = List.tabulate(count)(i => (i * 7 + base) % 101)
    val evens = xs.filter(x => x % 2 == 0).map(x => x + base)
    val total = evens.foldLeft(0L)((a, x) => a + x.toLong)
    val labelled = xs.collect { case x if x % 5 == 0 => "five" + x }
    val fs: List[Int => Int] = List(_ + base, _ * 2, x => x - base)
    val applied = xs.map(x => fs.foldLeft(x)((v, f) => f(v)))
    val sorted = applied.sortBy(x => -x)
    var acc = mix(total, labelled.length.toLong)
    acc = mix(acc, applied.foldLeft(0L)((a, x) => a + x.toLong))
    acc = mix(acc, sorted.head.toLong)
    acc = mix(acc, if xs.exists(_ == base) then 1L else 0L)
    acc = mix(acc, xs.zipWithIndex.map((x, i) => x * i).foldLeft(0L)((a, x) => (a + x.toLong) % modulus))
    acc

  def round(seed: Int): Long =
    var acc = composePass(400, seed)
    acc = mix(acc, partialPass(400))
    acc = mix(acc, counterPass(400))
    acc = mix(acc, iteratorPass(200))
    acc = mix(acc, bynamePass(400))
    acc = mix(acc, collectionPass(200, seed))
    acc

  def work(rounds: Int): Long =
    var acc = 0L
    var r = 0
    while r < rounds do
      acc = mix(acc, round(r * 3 + 1))
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
    if report then System.err.println(line("closures", n, times))
    println("closures checksum " + checksum)

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
