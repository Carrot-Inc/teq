// The harness's own check (tests/asymptotics.sh runs it beside the probes, never as one): an operation of a quadratic
// cost fails each of its three takes, and a linear one whose first take a stall puts over its bound passes on another
// take, every take on its line. The stall is the work's own time twenty times over, so that it is over the bound
// whatever the target's speed.
object HarnessSelfTest:
  def main(args: Array[String]): Unit =
    val base = if args.length > 0 then args(0).toInt else 200
    var sink = 0L
    Probe.check("planted quadratic", base, Probe.linear, n =>
      var i = 0
      while i < n do
        var j = 0
        while j < n do
          sink += j
          j += 1
        i += 1)
    var largest = 0
    Probe.check("planted stall once", base, Probe.linear, n =>
      val t0 = Probe.now()
      var i = 0
      while i < n * 200 do
        sink += i
        i += 1
      if n == base * 100 then
        largest += 1
        if largest == 1 then
          val until = Probe.now() + 20.0 * (Probe.now() - t0)
          while Probe.now() < until do sink += 1)
    println("sink " + (sink % 7))
    Probe.finish()
