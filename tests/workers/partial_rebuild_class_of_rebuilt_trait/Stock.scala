package inc
trait Show[T]:
  def show(t: T): String
  def twice(t: T) = show(t) + show(t)
object Stock:
  println("stock-init")
  val plain = "p" + 1
