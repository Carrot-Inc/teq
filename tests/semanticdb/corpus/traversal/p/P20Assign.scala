package p20
object As:
  var v = 1
  def setv(): Unit = v = 2
  class Prop:
    private var _x = 0
    def x = _x
    def x_=(n: Int): Unit = _x = n
  val p = Prop()
  def m(): Unit =
    p.x = 3
    var loc = 1
    loc = 2
    loc += 1
    As.v = 4
    v += 1
  val arr = Array(1)
  def u(): Unit = arr(0) = 1
