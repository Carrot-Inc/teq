/*
rules = ["file:../../support/FillClauses.scala"]
*/
// A parameter clause spans its parentheses, an empty one too (scalameta's; the review of `340567fdd`): `replaceTree`
// on `()` edits it, and a clause with parameters is left alone.
object Input {
  def f(): Int = 1
  def g(a: Int)(): Int = a
}
