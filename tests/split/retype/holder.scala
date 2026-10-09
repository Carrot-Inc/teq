package retype

/** A result type left to inference, whose body makes a class: the body is typed when the
  * signature is first asked for, in the middle of `keptBy`'s expansion in fold.scala. */
object Holder:
  def rule = new Rule:
    def keeps(word: String): Boolean = word.nonEmpty
