// A default of a later clause takes the earlier clause's argument, evaluated once: the argument
// bound before the call and passed to both, for a method and for a constructor.
object DefaultsCurried {
  def tick(): Int = { print("T"); 1 }
  def f(a: Int)(b: Int = a): Int = a + b
  class K(a: Int)(b: Int = a) { override def toString = s"K${a + b}" }
  def main(args: Array[String]): Unit = {
    println(f(tick())())
    println(new K(tick())())
  }
}
