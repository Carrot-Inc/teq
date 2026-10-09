package errs

/** Errors whose spans an edit changes that leaves the signatures what they were. */
class Spans:
  def wide(x: Box[List[Int]]): Int = 1
  def ticked(y: Absent): Int = 2
  def label: String = "spans"
