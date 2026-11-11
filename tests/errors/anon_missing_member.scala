// expect: 7:22: error: object creation impossible, since def size: Int in trait Named is not defined
// expect: 1 error found

trait Named:
  def name: String
  def size: Int
val missing: Named = new Named { def name: String = "n" }
