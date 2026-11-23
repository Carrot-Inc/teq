//> using platform jvm
// A call of a def by itself under a `return` in return position is the def's loop, as a call there
// without one is: a million calls deep on a thread of a small stack, no stack overflows.
object Countdown {
  final def inIf(n: Int): Int = if n == 0 then 0 else return inIf(n - 1)
  final def wrapsIf(n: Int): Int = return (if n == 0 then 0 else wrapsIf(n - 1))
  final def inCase(n: Int): Int = n match {
    case 0 => 0
    case _ => return inCase(n - 1)
  }
  final def inBlock(n: Int): Int = {
    val k = n - 1
    if n == 0 then 0 else return inBlock(k)
  }
  final def counted(n: Int, acc: Long): Long = if n == 0 then acc else return counted(n - 1, acc + 2L)
}

class Walker(val steps: Int) {
  final def walk(n: Int, other: Walker): Int = if n == 0 then other.steps else return walk(n - 1, other)
}

object Main {
  def main(args: Array[String]): Unit = {
    val n = 1000000
    val deep = new Thread(null, () => {
      println(Countdown.inIf(n))
      println(Countdown.wrapsIf(n))
      println(Countdown.inCase(n))
      println(Countdown.inBlock(n))
      println(Countdown.counted(n, 0L))
      println(new Walker(1).walk(n, new Walker(7)))
    }, "deep", 1L << 20)
    deep.start()
    deep.join()
  }
}
