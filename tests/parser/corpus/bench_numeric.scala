// Numeric loops: Int and Long arithmetic that wraps, Double arithmetic, while loops, tail
// recursion and an Int array sieve. Only integers are printed; the Double results are folded in
// as scaled Longs, since a Double prints as JavaScript prints it on the JS target.
object NumericBench:
  // Rounds, timed iterations and whether the per-iteration line is printed on stderr, when the
  // command line gives none; bench/runtime.sh compiles copies with its own values in this line,
  // since Scala.js gives main no arguments.
  val defaults = "150 1 0"

  val modulus = 1000000007L

  def mix(acc: Long, x: Long): Long = (acc * 31L + x) % modulus

  // A wrapping Int loop: multiplication that overflows, shifts and masks.
  def intLoop(seed: Int, steps: Int): Long =
    var x = seed | 1
    var acc = 0
    var i = 0
    while i < steps do
      x = x * 1103515245 + 12345
      acc = acc + (x >>> 7) - (x >> 11) + (x << 3)
      acc = acc ^ (acc >>> 13)
      i += 1
    (acc.toLong & 0xffffffffL)

  // The same in Long, which JavaScript has no primitive for: teq gives it a BigInt and Scala.js a
  // pair of Ints.
  def longLoop(seed: Long, steps: Int): Long =
    var x = seed | 1L
    var acc = 0L
    var i = 0
    while i < steps do
      x = x * 6364136223846793005L + 1442695040888963407L
      acc = acc + (x >>> 17) - (x >> 23)
      acc = acc ^ (acc >>> 29)
      i += 1
    acc % modulus

  // Double arithmetic: a dot product, a running mean and a square root, all of them exactly
  // rounded operations that the JVM and JavaScript agree on.
  def doubleLoop(steps: Int): Long =
    var dot = 0.0
    var mean = 0.0
    var i = 0
    while i < steps do
      val a = (i % 97).toDouble / 8.0
      val b = ((i * 3) % 89).toDouble / 4.0
      dot = dot + a * b - a / (b + 1.0)
      mean = mean + (a - mean) / (i + 1).toDouble
      i += 1
    val root = math.sqrt(dot * dot + mean * mean + 1.0)
    (dot * 64.0).toLong + (mean * 1024.0).toLong + (root * 16.0).toLong

  // Tail recursion, at a depth every platform takes without a deeper stack.
  def sumTo(n: Int, acc: Long): Long =
    if n <= 0 then acc else sumTo(n - 1, acc + n.toLong)

  def collatzSteps(start: Long, steps: Int): Int =
    if start == 1L || steps > 400 then steps
    else if start % 2L == 0L then collatzSteps(start / 2L, steps + 1)
    else collatzSteps(3L * start + 1L, steps + 1)

  // Non-tail recursion over a small tree of calls.
  def fib(n: Int): Long = if n < 2 then n.toLong else fib(n - 1) + fib(n - 2)

  // An Int array sieve: allocation, bounds-checked writes and a counting pass.
  def sieve(limit: Int): Long =
    val marks = new Array[Int](limit)
    var i = 2
    while i * i < limit do
      if marks(i) == 0 then
        var j = i * i
        while j < limit do
          marks(j) = 1
          j += i
      i += 1
    var count = 0
    var last = 0
    var k = 2
    while k < limit do
      if marks(k) == 0 then
        count += 1
        last = k
      k += 1
    count.toLong * 100000L + last.toLong

  // An Int array sorted in place by insertion, the shape a numeric kernel has.
  def insertionSort(size: Int): Long =
    val xs = new Array[Int](size)
    var x = 7
    var i = 0
    while i < size do
      x = x * 1103515245 + 12345
      xs(i) = x >>> 20
      i += 1
    var a = 1
    while a < size do
      val key = xs(a)
      var b = a - 1
      while b >= 0 && xs(b) > key do
        xs(b + 1) = xs(b)
        b -= 1
      xs(b + 1) = key
      a += 1
    var acc = 0L
    var c = 0
    while c < size do
      acc = (acc * 7L + xs(c).toLong) % modulus
      c += 1
    acc

  def round(seed: Int): Long =
    var acc = intLoop(seed, 2000)
    acc = mix(acc, longLoop(seed.toLong, 1000))
    acc = mix(acc, doubleLoop(1000))
    acc = mix(acc, sumTo(2000, 0L))
    acc = mix(acc, collatzSteps((seed % 1000).toLong + 3L, 0).toLong)
    acc = mix(acc, fib(18))
    acc = mix(acc, sieve(4096))
    acc = mix(acc, insertionSort(256))
    acc

  def work(rounds: Int): Long =
    var acc = 0L
    var r = 0
    while r < rounds do
      acc = mix(acc, round(r * 17 + 1))
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
    if report then System.err.println(line("numeric", n, times))
    println("numeric checksum " + checksum)

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
