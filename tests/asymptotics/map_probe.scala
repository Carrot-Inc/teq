// The immutable Map's costs: a chain of insertions, updates and removals, lookups, iteration,
// removal and re-insertion cycles, versions branching from one base, a version held across
// the updates made from it, and children branching from a parent at the compaction threshold,
// each at n, 10n and 100n.
object MapProbe:
  def main(args: Array[String]): Unit =
    val base = if args.length > 0 then args(0).toInt else 10000
    def built(n: Int): Map[Int, Int] =
      var m = Map.empty[Int, Int]
      var i = 0
      while i < n do
        m = m.updated(i * 7, i)
        i += 1
      m
    Probe.check("insert chain", base, Probe.logLinear, n => built(n))
    Probe.check("insert chain, string keys", base, Probe.logLinear, n =>
      var m = Map.empty[String, Int]
      var i = 0
      while i < n do
        m = m.updated("k" + i, i)
        i += 1)
    Probe.check("update chain", base, Probe.logLinear, n =>
      var m = built(n)
      var i = 0
      while i < n do
        m = m.updated(i * 7, -i)
        i += 1)
    Probe.measure("lookups", base, Probe.logLinear, n => (built(n), n), (m, n) =>
      var hits = 0
      var pass = 0
      while pass < 5 do
        var i = 0
        while i < n do
          if m.contains(i * 7) then hits += 1
          i += 1
        pass += 1
      if hits != 5 * n then println("FAIL lookups: " + hits + " hits of " + 5 * n))
    Probe.measure("iteration", base, Probe.linear, n => built(n), m =>
      var sum = 0L
      var pass = 0
      while pass < 5 do
        m.foreach(e => sum += e._2)
        pass += 1)
    Probe.check("remove chain", base, Probe.logLinear, n =>
      var m = built(n)
      var i = 0
      while i < n do
        m = m.removed(i * 7)
        i += 1
      if m.nonEmpty then println("FAIL remove chain: " + m.size + " left"))
    Probe.check("remove and reinsert cycles", base, Probe.logLinear, n =>
      var m = built(n)
      var i = 0
      while i < n do
        m = m.removed(i * 7).updated(i * 7, i)
        i += 1
      if m.size != n then println("FAIL cycles: size " + m.size))
    Probe.measure("branches from one base", base, Probe.logLinear, n => (built(n), n), (m, n) =>
      var i = 0
      var sum = 0L
      while i < n do
        sum += m.updated(i * 7, -1).size
        i += 1)
    Probe.measure("removals branching from one base", base, Probe.logLinear, n => (built(n), n), (m, n) =>
      var i = 0
      var sum = 0L
      while i < n do
        sum += m.removed(i * 7).size
        i += 1)
    Probe.check("held version read after updates", base, Probe.logLinear, n =>
      val held = built(n)
      var m = held
      var i = 0
      while i < n do
        m = m.updated(i * 7, -i)
        i += 1
      var hits = 0
      i = 0
      while i < n do
        if held(i * 7) == i then hits += 1
        i += 1
      if hits != n then println("FAIL held version: " + hits + " of " + n))
    Probe.check("Map.from and groupBy", base, Probe.logLinear, n =>
      val xs = List.tabulate(n)(i => i)
      val m = xs.map(i => (i, i)).toMap
      val g = xs.groupBy(_ % 1000)
      if m.size != n || g.size != (if n < 1000 then n else 1000) then println("FAIL from: " + m.size + " " + g.size))
    // A parent one removal short of the compaction threshold: the child that crosses it pays
    // the rebuild once (its time is the unit), and the m children after it must share that
    // rebuild through the memo, so their time together stays under a fifth of m units, where m
    // rebuilds would cost m units.
    val parentSize = base * 10
    var parent = built(parentSize)
    var removed = 0
    while 2 * (removed + 1) <= parentSize do
      parent = parent.removed(removed * 7)
      removed += 1
    val t0 = Probe.now()
    val first = parent.removed(removed * 7).size
    val unit = Probe.now() - t0
    val m = base / 10
    val t1 = Probe.now()
    var i = 0
    var sum = 0L
    while i < m do
      sum += parent.removed((removed + 1 + i) * 7).size
      i += 1
    val children = Probe.now() - t1
    val shared = children < unit * m.toDouble / 5.0
    println((if shared then "PASS " else "FAIL ") + "children of a parent at the compaction threshold: the first " + Probe.fmt(unit) + " ms, " + m + " more " + Probe.fmt(children) + " ms" + (if shared then "" else " (one rebuild each)"))
    if !shared then Probe.failures += 1
    if first != parentSize - removed - 1 || sum != m.toLong * (parentSize - removed - 1) then println("FAIL compaction: sizes " + first + " " + sum)
    Probe.finish()
