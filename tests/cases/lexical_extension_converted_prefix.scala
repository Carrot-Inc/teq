// An imported extension found through a conversion of its receiver (`R` to `S[Int]`) has its
// prefix's givens looked for at the converted receiver's type: `Need[Int]` has no instance, so
// that alternative fails and the other import's is selected (scalac `B`; the prefix read at `R`
// left `Need[T]` open, counted it found and made the two an ambiguity).
import scala.language.implicitConversions
trait Need[A]
class R
class S[A]
given Conversion[R, S[Int]] = _ => new S[Int]
object A:
  extension [T](s: S[T])(using Need[T]) def pick(x: Int): String = "A"
object B:
  extension (r: R) def pick(x: Int): String = "B"
object Main:
  import A.*
  import B.*
  def main(args: Array[String]): Unit = println((new R).pick(1))
