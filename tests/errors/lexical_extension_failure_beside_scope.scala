// expect: error: no given instance of type Need was found
// expect: 1 error found
// A lexical extension whose prefix fails beside candidates of the implicit scope that do not
// take the receiver: the lexical failure is the error, as dotty keeps it for the search
// (`FailedExtension`): the missing `Need` is named, not a bare "not a member".
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
