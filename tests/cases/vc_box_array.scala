// An array of a value class holds its boxes: an element stored, read, mapped and copied out.
class V(val x: Int) extends AnyVal
class S(val s: String) extends AnyVal
object Main {
  def upd(arr: Array[V], i: Int, v: V): Unit = arr(i) = v
  def main(args: Array[String]): Unit = {
    val arr = Array(new V(1), new V(2))
    println(arr(0).x); arr(1) = new V(5); println(arr.map(_.x).mkString(","))
    upd(arr, 0, new V(7)); println(arr(0).x); println(arr.length)
    println(arr.toList.map(_.getClass.getName)); println((arr(1): Any).getClass.getName)
    val ss = Array.fill(2)(new S("q")); ss(1) = new S("r"); println(ss.map(_.s).mkString)
    val any: Array[Any] = Array(new V(3), 3); println(any.map(_.getClass.getName).mkString(","))
  }
}
