import scala.collection.mutable.ArrayBuffer
class Box(var v: Int):
  def update(i: Int, x: Int): Unit = v = x
  def apply(i: Int): Int = v
@main def main(): Unit =
  val arr = Array(1, 2, 3)
  arr(0) += 1
  arr(1) = 5
  println(arr.toList)
  val buf = ArrayBuffer(1, 2)
  buf(0) += 10
  println(buf)
  val bx = Box(1)
  bx(0) = 7
  println(bx.v)
  bx(0) += 1
  println(bx.v)
  val m = scala.collection.mutable.Map("a" -> 1)
  m("a") += 1
  println(m)
