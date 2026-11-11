package ranked

/** The val the first given's type names, and two searches that rank the givens. */
object Vals:
  val x = new Derived
  def pick: Base = summon[Base]
  def again: String = summon[Base].name + "v"
