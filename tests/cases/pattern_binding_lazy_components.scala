// teq: --werror
// `Desugar.makePatDef` for a lazy pattern definition, a member's and a local's alike: the holder lazy
// and each variable a `def` reading its element (Desugar.scala 1659), so that a read adapts the
// element anew (a conversion runs at each read), a tuple literal's split re-evaluates its element at
// each read, and a holder evaluates its right side once. Above `MaxTupleArity` a variable reads
// `holder.apply(idx)`, zero-based (1650-1654), strict or lazy, member or local.
package patternbindinglazycomponents

import scala.language.implicitConversions

case class Raw(n: Int)
case class Cook(n: Int)

object Conv:
  var calls = 0
  given Conversion[Raw, Cook] = r => { calls += 1; Cook(r.n + calls) }

import Conv.given

def pairOf(n: Int): (Int, Int) = (n, n * 10)

object Members:
  lazy val (_, b: Cook) = (1, Raw(2))
  val (_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, x) = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23)
  lazy val (_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, y) = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23)

object Main:
  def main(args: Array[String]): Unit =
    println((Members.b.n, Members.b.n, Conv.calls))
    lazy val (_, c: Cook) = (1, Raw(5))
    println((c.n, c.n, Conv.calls))
    var n = 0
    def f(): Int = { n += 1; n }
    lazy val (p, q) = (f(), f())
    println((p, p, q, q, n))
    lazy val (_, r) = (f(), f())
    println((r, r, n))
    lazy val (s, t) = pairOf(f())
    println((s, t, s, n))
    lazy val (_, w: Long) = (1, 2)
    println(w + 1L)
    val (_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, z) = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23)
    lazy val (_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _, zz) = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23)
    println((Members.x, Members.y, z, zz))
