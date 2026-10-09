// teq: --threads 1
// expect: 20:21: error: type mismatch
// expect: 1 error found
// At one worker `H.g`'s body is typed on demand during the check of the inline `I.f`, which drains
// what it reported and puts back what stays; no promoted range keeps a position past the drain
// (`state::diags_cut_at`), so the first extension alternative's own mismatch on `5` is its failure
// and the second applies, as in scalac and master. The one error is the last line's.
import scala.compiletime.*

trait Ops:
  extension [T](x: Int)
    def combine(y: String, z: String): String = "s " + y + z
  extension [T](x: Int)
    def combine(y: String, z: Int): String = "i " + y + z
object O extends Ops

@main def run(): Unit =
  println(O.combine[Unit](1)(I.f, 5))
  val n: Int = 1
  val bad: String = n

class Box[A]
object I:
  transparent inline def f: String =
    inline if false then
      val a = { 1; 2 }
      "x"
    else
      val s = H.g
      summonFrom { case b: Box[y] if constValue[y] == 3 => s; case _ => "none" }

object H:
  def g = { 2; "g" }
