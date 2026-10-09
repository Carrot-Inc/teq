package retype

/** No macro: an argument of an inline call that the interpreter evaluates, and an inline
  * call that is the first to ask for the signature of `Holder.rule`. */
object Fold:
  val kept = keptBy("ab")
  def label: String = "fold"
  def sum: Int = doubled(List(1, 2, 3).sum)
