// Consecutive imports of an inline body's block and consecutive local inline methods of a block,
// each pickled before the statement they stand before, none in the place of another: scalac's
// expansion from teq's pickle finds the given `A.given` brings (`1`, `0` without it).
object A { given Int = 7 }
object B { val unused = 0 }

object Lib:
  inline def pick: Int =
    import A.given
    import B.unused
    scala.compiletime.summonFrom { case _: Int => 1; case _ => 0 }
  def twice(x: Int): Int =
    inline def one(y: Int): Int = y + 1
    inline def two(y: Int): Int = one(y) * 2
    two(x)

object BlockImports:
  def main(args: Array[String]): Unit =
    println(Lib.pick)
    println(Lib.twice(3))
