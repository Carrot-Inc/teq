// The immutable Set's costs: a chain of insertions and removals, membership, iteration and
// versions branching from one base, each at n, 10n and 100n.
object SetProbe:
  def main(args: Array[String]): Unit =
    val base = if args.length > 0 then args(0).toInt else 10000
    def built(n: Int): Set[Int] =
      var s = Set.empty[Int]
      var i = 0
      while i < n do
        s = s + i * 3
        i += 1
      s
    Probe.check("insert chain", base, Probe.logLinear, n => built(n))
    Probe.measure("membership", base, Probe.logLinear, n => (built(n), n), (s, n) =>
      var hits = 0
      var pass = 0
      while pass < 5 do
        var i = 0
        while i < n do
          if s.contains(i * 3) then hits += 1
          i += 1
        pass += 1
      if hits != 5 * n then println("FAIL membership: " + hits + " of " + 5 * n))
    Probe.measure("iteration", base, Probe.linear, n => built(n), s =>
      var sum = 0L
      var pass = 0
      while pass < 5 do
        s.foreach(x => sum += x)
        pass += 1)
    Probe.check("remove chain", base, Probe.logLinear, n =>
      var s = built(n)
      var i = 0
      while i < n do
        s = s - i * 3
        i += 1
      if s.nonEmpty then println("FAIL remove chain: " + s.size + " left"))
    Probe.measure("branches from one base", base, Probe.logLinear, n => (built(n), n), (s, n) =>
      var i = 0
      var sum = 0L
      while i < n do
        sum += (s + (i * 3 + 1)).size
        i += 1)
    Probe.check("toSet and union", base, Probe.logLinear, n =>
      val s = List.tabulate(n)(i => i).toSet
      if (s ++ built(n)).size != n + n - (n + 2) / 3 then println("FAIL union: " + (s ++ built(n)).size))
    Probe.finish()
