object P1:
  val xs = List(1, 2)
  def line(using l: sourcecode.Line): Int = l.value
  def larger: Int = math.max(1, 2)
  val b = new libb.B
  def n: Int = b.twiceHello
  val c = new libb.Cases
  def m: Int = c.helper(x = 4)
  def buffered(b: scala.collection.mutable.ListBuffer[Int]): Int = b.length
