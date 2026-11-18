// expect: 17:57: error: value pick is not a member of R.
// expect: 1 error found
// A lexical extension whose prefix fails beside candidates of the implicit scope that do not take the receiver: the
// lexical failure is the error, as dotty keeps it for the search (`FailedExtension`): scalac 3.8.4 reports `value pick
// is not a member of R.` at the selection, the extension it tried and the missing `Need` in the lines after it.
trait Need
class R
class Q
trait Ops:
  extension (q: Q) def pick: String
object R:
  extension (q: Q) def pick: String = "companion of another receiver"
  given Ops with
    extension (q: Q) def pick: String = "given of another receiver"
object Main:
  extension (r: R)(using Need) def pick: String = "lexical"
  def main(args: Array[String]): Unit = println((new R).pick)
