class C { def f(): Int = { println("f"); 1 }; def g(x: Int): Int = x }
object P {
  var made = 0
  def mk(): C = { made += 1; new C }
  def main(args: Array[String]): Unit = {
    val a = mk().f _
    val b = mk().g _
    println(made)
    println(a() + a() + b(2))
    println(made)
  }
}
