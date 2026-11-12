// Two conversions that both give the selected member are the view
// search's ambiguity, one arm with an extension methods' ambiguity
// (Typer.scala:4346-4353): the qualifier's mismatch at the qualifier, each
// candidate by `refStr` ("given instance c1 in object Main"). teq is to report
// it so. Queued for push 6 (`AmbiguousImplicits.qualify`'s conversion arms).
import scala.language.implicitConversions
class R
class HasPick:
  def pick: String = "h"
object Main:
  given c1: Conversion[R, HasPick] = _ => new HasPick
  given c2: Conversion[R, HasPick] = _ => new HasPick
  def main(args: Array[String]): Unit =
    println((new R).pick)
