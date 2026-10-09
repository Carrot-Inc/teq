// The Vector's costs: chains of appends, prepends, updates, front drops and end cuts, indexed
// reads, iteration and versions branching from one base, each at n, 10n and 100n.
object VectorProbe:
  def main(args: Array[String]): Unit =
    val base = if args.length > 0 then args(0).toInt else 10000
    def built(n: Int): Vector[Int] =
      var v = Vector.empty[Int]
      var i = 0
      while i < n do
        v = v :+ i
        i += 1
      v
    Probe.check("append chain", base, Probe.linear, n => built(n))
    Probe.check("prepend chain", base, Probe.logLinear, n =>
      var v = Vector.empty[Int]
      var i = 0
      while i < n do
        v = i +: v
        i += 1
      if v.length != n || v(0) != n - 1 then println("FAIL prepend chain: " + v.length))
    Probe.check("update chain", base, Probe.logLinear, n =>
      var v = built(n)
      var i = 0
      while i < n do
        v = v.updated(i, -i)
        i += 1
      if v(n - 1) != -(n - 1) then println("FAIL update chain"))
    Probe.measure("indexed reads", base, Probe.logLinear, n => (built(n).updated(0, 0), n), (v, n) =>
      var sum = 0L
      var pass = 0
      while pass < 5 do
        var i = 0
        while i < n do
          sum += v(i)
          i += 1
        pass += 1)
    Probe.measure("iteration", base, Probe.linear, n => built(n).updated(0, 0), v =>
      var sum = 0L
      var pass = 0
      while pass < 5 do
        v.foreach(x => sum += x)
        val it = v.iterator
        while it.hasNext do sum += it.next()
        pass += 1)
    Probe.check("front drop chain", base, Probe.logLinear, n =>
      var v = built(n)
      var i = 0
      while v.nonEmpty do
        i += v.head
        v = v.tail)
    Probe.check("end cut chain", base, Probe.logLinear, n =>
      var v = built(n)
      while v.nonEmpty do v = v.init)
    Probe.check("queue", base, Probe.logLinear, n =>
      var v = built(100)
      var i = 0
      while i < n do
        v = v.tail :+ i
        i += 1
      if v.length != 100 then println("FAIL queue: " + v.length))
    Probe.measure("updates branching from one base", base, Probe.logLinear, n => (built(n), n), (v, n) =>
      var i = 0
      var sum = 0L
      while i < n do
        sum += v.updated(i, -1)(i)
        i += 1)
    Probe.measure("appends branching from one base", base, Probe.logLinear, n => (built(n), n), (v, n) =>
      var i = 0
      var sum = 0L
      while i < n do
        sum += (v :+ i).length
        i += 1)
    Probe.measure("slices", base, Probe.logLinear, n => (built(n), n), (v, n) =>
      var i = 0
      var sum = 0L
      while i < n do
        sum += v.slice(i / 2, i / 2 + 40).length
        i += 1)
    Probe.finish()
