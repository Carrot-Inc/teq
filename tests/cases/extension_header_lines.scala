// An extension's methods are the indented lines after its header, wherever the header's last clause closes: on a
// line of its own, at the margin, after a clause of brackets, a using clause or a parenthesis of the receiver's
// type broken over lines. A blank line or a comment may stand between the header and the methods; a header after
// a `;`, inside a block and in braces opened on its line reads the same. scalac prints the same lines.
object OwnLine:
  extension (
      s: String
  )
    def twice: String = s + s

object Brackets:
  extension [A,
      B](p: (A, B))
    def first: A = p._1

object BlankLine:
  extension (
      s: String
  )

    def thrice: String = s + s + s

object CommentLine:
  extension (
      s: String
  )
    // the methods
    def quoted: String = "'" + s + "'"

object CommentAfter:
  extension (
      s: String
  ) // the receiver
    def bracketed: String = "[" + s + "]"

object UsingClause:
  extension (s: String)(using
      n: Int
  )
    def times: String = s * n

object AfterSemicolon:
  val one = 1; extension (s: String)
    def upper: String = s.toUpperCase

object NestedParens:
  extension (p: (Int,
      Int))
    def sum: Int = p._1 + p._2

object AtMargin:
  extension (
      s: String
)
    def lowered: String = s.toLowerCase

object InBlock:
  def outer: Int =
    extension (
        x: Int
    )
      def next: Int = x + 1
    1.next

object InBraces { extension (x: Int
)
    def prev: Int = x - 1
}

object InBracesBracket { extension [T
](x: T)
    def same: T = x
}

object InBracesParam { extension (x:
  Int)
    def doubled: Int = x * 2
}

object InBracesComment { extension (x: Int /*
 comment */)
    def tripled: Int = x * 3
}

object InBracesBracketEndsLine { extension [
    T](x: T)
    def itself: T = x
}

@main def main(): Unit =
  import OwnLine.*, Brackets.*, BlankLine.*, CommentLine.*, CommentAfter.*, UsingClause.*, AfterSemicolon.*
  import NestedParens.*, AtMargin.*, InBraces.*, InBracesBracket.*, InBracesParam.*, InBracesComment.*
  import InBracesBracketEndsLine.*
  given Int = 2
  println("ab".twice)
  println((1, "x").first)
  println("ab".thrice)
  println("ab".quoted)
  println("ab".bracketed)
  println("ab".times)
  println("ab".upper)
  println((1, 2).sum)
  println("AB".lowered)
  println(InBlock.outer)
  println((5.prev, "t".same, 5.doubled, 5.tripled, 'i'.itself))
