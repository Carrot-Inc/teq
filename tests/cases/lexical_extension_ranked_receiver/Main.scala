// A given whose extension's receiver class rules the receiver out is passed over before it is
// instantiated: its transparent expansion, a macro's run, is not made for a candidate that cannot
// apply (scalac `0`, `1`; an implementation that expanded it while ranking printed `0`, `2`):
// `M.next` counts the expansions.
object Main:
  import G.given
  def main(args: Array[String]): Unit =
    println((new R).pick)
    println(M.next)
