/*
rules = ["file:../../support/ReplaceParenthesized.scala"]
*/
// An infix pattern's parenthesized operand is in its argument clause, a refinement's body starts at its `{` after a
// parenthesized parent, and a named tuple's field spans its parenthesized value, as scalameta's: each replaced, the
// file types after.
object Operands:
  def first(xs: List[Int]): Int = xs match
    case h :: (t) => h
    case _ => 0
  type Refined = (AnyRef) { type T }
  val pair = (a = (1), b = "b")
