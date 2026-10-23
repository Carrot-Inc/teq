/*
rules = ["file:../../support/DumpLayout.scala"]
*/
// An indented body whose region ends with a comment on its own line within it: scalameta 4.17.4 keeps a `Term.Block`
// over a statement alone there, to the comment's end (`isPrecededByDetachedComment`), and a block of statements ends
// there too; a comment outside the region leaves the body as it is.
object DetachedComments:
  val marker = 0 // assert: DumpLayout.layout
  val x = 1
  def f =
    x
    // trailing
  def g =
    x
  // outside
  def h(n: Int) = n match
    case 1 =>
      x
      // in a case
    case _ => 2
  def i(c: Boolean) =
    if c then
      x
      /* in a then */
    else 2
  extension (a: Int)
    def double = a * 2
    // in an extension
  def k =
    val y = x
    y
    // after statements
  val j = 1
