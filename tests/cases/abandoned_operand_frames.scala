// A closure over a block's frame already evaluated as an argument when the next argument throws, at every
// iteration: the run goes on, and in the interpreter's lowered code the abandoned operand is dropped before
// the block's frame is left so that its local def's cycle is cut (tests/asymptotics.sh measures the memory
// of the same program, tests/asymptotics/frame_cycle.scala, at 20 iterations and at 200).
object Main:
  def sink(f: () => Int, n: Int): Int = f() + n
  def fail(i: Int): Int =
    val payload = "x" * 100000
    def local(): Int = payload.length + i
    sink(() => local(), throw new RuntimeException("stop " + i))
  def main(args: Array[String]): Unit =
    var i = 0
    var caught = 0
    while i < 50 do
      try fail(i) catch case e: RuntimeException => if e.getMessage == "stop " + i then caught += 1
      i += 1
    println(s"$i iterations, $caught caught")
