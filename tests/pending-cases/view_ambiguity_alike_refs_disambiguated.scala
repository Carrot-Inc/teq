// Two candidates of one name and one owner's name: the ambiguity's message, the qualifier's mismatch
// (Typer.scala:4346-4353), is shown by the explaining printer, which tells the two symbols apart by a superscript and
// explains each in a `where:` line (`ops` and `ops²`, "is a object in object T"). teq reports its own ambiguity at the
// selection, both by the bare name ("both ops and ops provide pick on R"); master (efddb38de3) and the head
// (61c1af7941) alike. Queued: the message push 6's (typeMismatch whole), the superscripts the rendering owner's (the
// printers' disambiguation, below the implicits port's boundary; push 5's interim review, finding 8).
trait Ops:
  extension (r: R) def pick: String
object A:
  trait T
  object T:
    given ops: Ops with
      extension (r: R) def pick: String = "A"
object B:
  trait T
  object T:
    given ops: Ops with
      extension (r: R) def pick: String = "B"
class R extends A.T, B.T
object Main:
  def main(args: Array[String]): Unit = println((new R).pick)
