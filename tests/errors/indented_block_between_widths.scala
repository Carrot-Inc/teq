// expect: 9:4: error: The start of this line does not match any of the previous indentation widths.
// absent: 9:4: error: expected end of statement
// A selection that continues an indented block stands at the width of a region around the block or far from the
// widths on both sides of it: one column off the def's width and the block's is neither. scalac: "The start of
// this line does not match any of the previous indentation widths", and no other error.
object O:
  def f: Int =
    List(1, 2)
   .head
