// Top-level definitions reached from an object of their file, outside the file's own top-level
// definitions: every call of a def and every read of an eager val checks the file's initialiser
// first, a call of nothing once it has run. A call, a call through the
// function value made from the def, and a read of the val through its accessor.
val table: Array[Int] = Array.tabulate(64)(i => i * 7 + 3)

def mix(acc: Int, x: Int): Int = (acc * 31) ^ x

object TopLevelBench:
  // Rounds, timed iterations and whether the per-iteration line is printed on stderr, when the
  // command line gives none; bench/runtime.sh compiles copies with its own values in this line,
  // since Scala.js gives main no arguments.
  val defaults = "360 1 0"

  val perRound = 20000

  def calls(n: Int): Int =
    var acc = 1
    var i = 0
    while i < n do
      acc = mix(acc, i)
      i += 1
    acc

  def values(n: Int, f: (Int, Int) => Int): Int =
    var acc = 1
    var i = 0
    while i < n do
      acc = f(acc, i)
      i += 1
    acc

  def reads(n: Int): Int =
    var acc = 0
    var i = 0
    while i < n do
      acc = acc + table(i & 63)
      i += 1
    acc

  def work(rounds: Int): Long =
    val n = rounds * perRound
    calls(n).toLong + values(n, mix).toLong * 3L + reads(n).toLong * 7L

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
    if report then System.err.println(line("toplevel", n, times))
    println("toplevel checksum " + checksum)

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
