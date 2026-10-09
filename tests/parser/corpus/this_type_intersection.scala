// A member returning this.type seen through a receiver of an intersection type, a trait with
// a self type among its parts, on a stable path and on a new instance.
class C:
  def name = "c"
trait T:
  self: C =>
  def f: this.type = this
trait U:
  def g: this.type = this
object Main:
  def main(args: Array[String]): Unit =
    val x: C & T = new C with T
    val y: C & T = x.f
    val z: C & U = (new C with U).g
    val w: C & T = List(1).foldLeft(new C with T)((acc, _) => acc.f)
    println(y.name + z.name + w.name)
