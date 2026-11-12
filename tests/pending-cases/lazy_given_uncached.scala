// Pending on uncache-given-aliases (queued): teq keeps every given alias in its thread-safe holder, evaluating the
// right-hand side once, until dotty's UncacheGivenAliases is ported whole (`needsCache` over the right-hand side's
// type, `Types.isStable` with its recursive branches); the expectation is scalac 3.8.4's.
// Given aliases dotty's UncacheGivenAliases leaves uncached, a def evaluating `this`, `this.y` of a stable member or a
// prefix-less stable `y` at each use, beside one it keeps in a lazy val (Ordering.Int): the values are the same. The
// class files: scalac writes `self()` and `same()` with no `self$lzy1` or `same$lzy1` holder, `kept$lzy1` beside
// `kept()`; teq writes a holder for each.
class Ctx(val name: String):
  var reads = 0
  given self: Ctx = this
  given same: String = this.name
  given kept: Ordering[Int] = { reads += 1; Ordering.Int }

object Main:
  def local(n: Int): Int =
    given g: Int = n
    summon[Int] + summon[Int]

  def main(args: Array[String]): Unit =
    val c = new Ctx("ctx")
    import c.given
    println(summon[Ctx].name + " " + summon[String] + " " + summon[Ordering[Int]].compare(1, 2) + " " + summon[Ordering[Int]].compare(2, 1))
    println(s"kept computed ${c.reads} time(s)")
    println(local(21))
