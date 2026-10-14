// A block's frame whose local def and 1 MB payload a closure holds, the closure an argument already
// evaluated when the next argument throws: the frame is let go with its cycle cut on the way out of
// every iteration, in the interpreter's lowered code as in its walker (tests/asymptotics.sh measures
// the process's peak memory after 20 iterations and after 200).
object Main:
  def sink(f: () => Int, n: Int): Int = f() + n
  def fail(): Int =
    val payload = "x" * 1000000
    def local(): Int = payload.length
    sink(() => local(), throw new RuntimeException("stop"))
  def main(args: Array[String]): Unit =
    val count = if args.isEmpty then 200 else args(0).toInt
    var i = 0
    while i < count do
      try fail() catch case _: RuntimeException => ()
      i += 1
    println(i)
