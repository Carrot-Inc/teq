package una

// A method nothing downstream calls: the products build leaves it out, as the whole build does.
object Lib:
  def used: String = "used"
  def never: String = "NEVER-CALLED"
