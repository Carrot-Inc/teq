// The harness of the asymptotic probes: an operation run at three sizes a decade apart, the
// smaller two as the minimum of several runs, and the growth from the smallest to the largest
// compared with the bound the operation's cost allows: 600 for a linear cost and 1000 for
// n log n, where the two decades give 100 and about 150, the rest being room for a loaded
// machine and for the caches the smallest size fits in. An operation whose first decade
// already grows past 60 is not run at the largest size, so a copy per step (100 a decade)
// fails within seconds.
object Probe:
  var failures = 0
  def now(): Double = System.nanoTime().toDouble / 1000000.0
  def timed(reps: Int, body: () => Unit): Double =
    var best = 1.0e18
    var r = 0
    while r < reps do
      val t0 = now()
      body()
      val t = now() - t0
      if t < best then best = t
      r += 1
    best
  def check(name: String, base: Int, bound: Double, op: Int => Unit): Unit = measure[Int](name, base, bound, n => n, op)
  // `prepare` builds the operation's input outside the timing. An operation whose take reads over its bound is taken
  // twice more and passes where one take is within it: a stall of the machine (the collector, a neighbour) strikes
  // one take, a cost past the bound all three. Its line gives every take; the bound is the same for each.
  def measure[T](name: String, base: Int, bound: Double, prepare: Int => T, op: T => Unit): Unit =
    op(prepare(base))
    val first = take(base, bound, prepare, op)
    if first._1 then println("PASS " + name + ": " + first._2)
    else
      val takes = Vector(first, take(base, bound, prepare, op), take(base, bound, prepare, op))
      val within = takes.exists(_._1)
      if !within then failures += 1
      println((if within then "PASS " else "FAIL ") + name + ": " + takes.indices.map(i => "take " + (i + 1) + " " + takes(i)._2).mkString("; "))
  // One take: the three sizes, or the first two where the first decade already grows past 60; whether it is within
  // the bound, and its line.
  def take[T](base: Int, bound: Double, prepare: Int => T, op: T => Unit): (Boolean, String) =
    val t0 = timedOn(5, prepare(base), op)
    val t1 = timedOn(3, prepare(base * 10), op)
    val floor = if t0 < 0.05 then 0.05 else t0
    if t1 / floor > 60.0 then
      (false, fmt(t0) + " ms at " + base + ", " + fmt(t1) + " ms at " + base * 10 + " (x" + fmt(t1 / floor) + ")")
    else
      val t2 = timedOn(1, prepare(base * 100), op)
      val ratio = t2 / floor
      (ratio <= bound, fmt(t0) + " / " + fmt(t1) + " / " + fmt(t2) + " ms at " + base + " / " + base * 10 + " / " + base * 100 + ", x" + fmt(ratio) + " of " + fmt(bound))
  def timedOn[T](reps: Int, input: T, op: T => Unit): Double = timed(reps, () => op(input))
  def fmt(x: Double): String = (Math.round(x * 100.0).toDouble / 100.0).toString
  // Ten times the size costs ten times for a linear operation and about fifteen for one in
  // n log n; a memory that no longer fits a cache and the collector add to both, a copy per
  // step a hundredfold per decade.
  def linear: Double = 600.0
  def logLinear: Double = 1000.0
  def finish(): Unit = println(if failures == 0 then "all probes passed" else failures.toString + " probes failed")
