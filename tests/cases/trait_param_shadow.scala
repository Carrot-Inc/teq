trait Numbered(val num: Int):
  def fromTrait = num
class Same(num: Int) extends Numbered(num + 1):
  def fromClass = num
class Hidden(n: Int) extends Numbered(n):
  private val num = n * 10
  def fromClass = num
@main def run(): Unit =
  val s = Same(1)
  println(s.fromClass + " " + s.fromTrait + " " + s.num)
  val h = Hidden(2)
  println(h.fromClass + " " + h.fromTrait + " " + (h: Numbered).num)
