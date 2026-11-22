package pmenum

// A Java enum the JDK nests in a class: the member enum is the pattern-match target.
object ByState:
  def fresh(t: java.lang.Thread.State): Int = t match
    case java.lang.Thread.State.NEW => 1
    case _ => 0
