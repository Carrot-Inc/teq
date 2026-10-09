import scala.quoted.*

// Initialisers that make values and change them: an object's initialiser filling an array it
// made, and a lazy val's initialiser of an instance a run makes filling a buffer it made. What an
// initialiser makes is its own, so the runs stay on their workers.
object Table:
  val squares: Array[Int] =
    val xs = new Array[Int](8)
    var i = 0
    while i < 8 do
      xs(i) = i * i
      i += 1
    xs

class Summed(k: Int):
  lazy val total: Int =
    val buf = scala.collection.mutable.ArrayBuffer.empty[Int]
    buf += k
    buf += Table.squares(k % 8)
    buf.sum

object M:
  def sum(e: Expr[Int])(using Quotes): Expr[Int] =
    val k = e.valueOrAbort
    Expr(Summed(k).total + Table.squares(k % 8))

inline def ms(inline k: Int): Int = ${ M.sum('k) }
