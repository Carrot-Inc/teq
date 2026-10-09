import java.nio.CharBuffer

// A buffer's ranges are checked where they are made: `wrap` of an array's part and
// `subSequence` throw for a range outside what there is.
@main def main(): Unit =
  def attempt(what: String)(f: => Any): Unit =
    try println(s"$what ${f}") catch case _: IndexOutOfBoundsException => println(s"$what out of bounds")
  attempt("sub")(CharBuffer.wrap(Array('a')).subSequence(0, 2).limit())
  attempt("sub-neg")(CharBuffer.wrap(Array('a')).subSequence(-1, 1))
  attempt("sub-rev")(CharBuffer.wrap("ab").subSequence(2, 1))
  attempt("wrap")(CharBuffer.wrap(Array('a'), 0, 2).limit())
  attempt("wrap-neg")(CharBuffer.wrap(Array('a'), -1, 1).limit())
  attempt("ok")(CharBuffer.wrap(Array('a', 'b', 'c'), 1, 2).subSequence(0, 1).toString)
