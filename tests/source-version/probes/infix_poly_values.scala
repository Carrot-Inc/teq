class C {
  val f: [A] => A => A = [A] => (a: A) => a
  def g: [A] => A => A = [A] => (a: A) => a
  def q: [A] => (A, A) => A = [A] => (a: A, b: A) => b
  infix def h(x: Int): [A] => A => A = [A] => (a: A) => a
  val k: Int => Int = x => x + 1
  val d: (x: Int) => Int = x => x + 2
}
object P {
  def main(args: Array[String]): Unit = {
    val c = new C
    println(c f 1)
    println(c g "g")
    println(c q (1, 2))
    println((c h 1)(3))
    println(c k 1)
    println(c d 1)
  }
}
