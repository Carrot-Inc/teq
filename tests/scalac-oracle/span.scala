// A member missing from a parenthesized receiver over three lines: scalac's point is on the last line, its span
// starts on the first, where teq reports it.
object Span:
  val x = (
    "hello"
  ).missing
