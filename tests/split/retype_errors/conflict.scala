package errs

trait Left:
  def m: Int = 1

trait Right:
  def m: Int = 2

/** Two concrete members of one name inherited. */
class Both extends Left with Right:
  def label: String = "both"
