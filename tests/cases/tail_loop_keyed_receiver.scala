// teq: --wconf msg=Non.local.returns.are.no.longer.supported:s
final class Counter(val step: Int) {
  def sum(n: Int, acc: Int): Int = {
    println(s"$step $n $acc")
    List(3).foreach { end => if n >= end then return acc + step }
    new Counter(step + 2).sum(n + 1, acc + step)
  }
}
object Main {
  def main(args: Array[String]): Unit = println(new Counter(1).sum(0, 10))
}
