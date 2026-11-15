package inc
trait Marked:
  self: Stock.api.Request =>
  def mark: Int = n + 1
given secondShow: Show[Marked] = new Show[Marked] { def show = "marked" }
object Main:
  def main(args: Array[String]): Unit = println(read + summon[Show[Marked]].show)
