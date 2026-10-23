package p08
case class CC(a: Int, var b: String = "x")(c: Int):
  def d = a + c
case object CO
object Use:
  val cc = CC(1)(2)
  val cp = cc.copy(a = 3)(4)
  val CC(x, y) = cc: @unchecked
