object P {
  def n: Int = 3
  val x = 1
  def byName(y: => Int) = y _
  val a = x _
  val b = n _
}
