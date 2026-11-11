// A view search whose argument is `Nothing` or `Null` tries no candidate (`TooUnspecific`, Implicits.scala:1684-1686),
// ambiguous as an ambiguity is: the qualifier's mismatch with the note that no conversion was tried, against the
// prototype the search revealed (`deepenProto`), even where one given alone provides the extension (scalac 3.8.4 on
// each line). teq takes the given's extension: master (efddb38de3) and the head (61c1af7941) compile it. Queued: push 6
// (the search's `TooUnspecific` with typeMismatch whole and the printer's prototype arms).
trait Ops:
  extension (r: Any) def pick: String
object Main:
  given a: Ops with
    extension (r: Any) def pick: String = "a"
  def main(args: Array[String]): Unit =
    println((null: Null).pick)
    println((??? : Nothing).pick)
    val x = (null: Null).pick
