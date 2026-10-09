// The givens that provide an extension are ranked each in an attempt of its own, kept as dotty
// keeps a success's state: the one preferred, a transparent given whose instance runs a macro,
// is instantiated once, its macro run once (scalac `1`, `2`; a probe retracted and the winner
// instantiated again ran it twice, `2`, `3`).
import Outer.given
object Main:
  transparent inline given inner: Ops = new Ops { def k: Int = M.next }
  def main(args: Array[String]): Unit =
    println((new R).pick)
    println(M.next)
