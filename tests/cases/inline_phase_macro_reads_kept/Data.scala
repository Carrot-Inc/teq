// An object whose initialiser holds plain inline calls (the lean std's `Array.apply` is one), in a file
// without quotes, which a transparent macro's implementation reads while the program is typed.
object Kept:
  val items = Array(1, 2, 3)
  def total: Int = items.sum + Array(10, 20).sum
