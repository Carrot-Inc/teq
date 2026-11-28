// teq: --wconf msg=Non.local.returns.are.no.longer.supported:s
object Main {
  def f(a: Int, b: Int, delta: Int): Int = {
    println(s"$a $b $delta")
    List(0).foreach { zero => if delta < zero then return a + delta }
    if a == b then b else f(b, a, a - b)
  }
  def main(args: Array[String]): Unit = println(f(8, 3, 0))
}
