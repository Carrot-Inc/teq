package pa

case class Item(name: String)(val weight: Int)(using val unit: String)
case class Code(n: Int)
object Code:
  def unapply(c: Code): Option[Int] = if c.n > 0 then Some(c.n * 10) else None
object Pick:
  def unapply(s: String): Option[Int] = Some(s.length)
  def unapply(n: Int): Option[Int] = Some(n + 1)
def twice(x: => Int): Int = x + x
