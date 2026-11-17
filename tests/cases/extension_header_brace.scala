// The braces around an extension's methods may open on the line after its header, at the header's width or wider,
// after a header of one line or of several. scalac prints the same lines.
object NextLine:
  extension (s: String)
  {
    def twice: String = s + s
  }

object NextLineWider:
  extension (s: String)
      {
        def thrice: String = s + s + s
      }

object OneLineBraces:
  extension (x: Int)
  { def next: Int = x + 1 }

object WiderOneLineBraces:
  extension (x: Int)
    { def prev: Int = x - 1 }

object AfterBrokenHeader:
  extension (
    x: Int
  )
    { def doubled: Int = x * 2 }

@main def main(): Unit =
  import NextLine.*, NextLineWider.*, OneLineBraces.*, WiderOneLineBraces.*, AfterBrokenHeader.*
  println("ab".twice)
  println("ab".thrice)
  println((5.next, 5.prev, 5.doubled))
