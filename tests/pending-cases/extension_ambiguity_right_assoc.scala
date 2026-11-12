// A right-associative operator's view search is on its right
// operand, dotty's qualifier: the ambiguity is that operand's
// mismatch and names its type, among the givens in scope as in the
// operand's implicit scope (Typer.scala:4346-4353; scalac 3.8.4
// at both right operands). Queued: push 6 (typeMismatch whole).
class R
class S
trait Ops:
  extension (r: R) def *:: (l: List[Int]): String
trait SOps:
  extension (s: S) def *:: (t: S): String
object S:
  given c: SOps with
    extension (s: S) def *:: (t: S): String = "c"
  given d: SOps with
    extension (s: S) def *:: (t: S): String = "d"
object Main:
  given a: Ops with
    extension (r: R) def *:: (l: List[Int]): String = "a"
  given b: Ops with
    extension (r: R) def *:: (l: List[Int]): String = "b"
  def main(args: Array[String]): Unit =
    println((new R) *:: List(1))
    val s1 = new S
    println(s1 *:: (new S))
