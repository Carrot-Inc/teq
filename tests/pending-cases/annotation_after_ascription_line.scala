// An annotated definition on the line after an ascription's
// annotation (`e: @unchecked`) is a statement of its own: dotty's
// `annotations(skipNewLines)` (Parsers.scala:3603 at 3.8.4) skips
// line breaks for a definition's annotations alone; an ascription's
// (`skipNewLines = false`) end at the scanner's NEWLINE before the next line's `@`.
object O:
  val x = 1: @unchecked
  @deprecated val annotated = 1
  def f = annotated

@main def run(): Unit = println(O.f)
