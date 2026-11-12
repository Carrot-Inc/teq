// A given with parameters is a class dotty leaves extensible
// (`Desugar`'s `givenDef` sets no `Final`), so its method's self call
// is a virtual call that TailRec does not rewrite (`isEffectivelyFinal`
// false): a million-deep sum overflows the stack on the JVM (scalac
// `overflow`). Waiting on a given class's flags as dotty sets them.
trait Sum:
  def sum(n: Int, acc: Long): Long

given withParams(using k: Int): Sum with
  def sum(n: Int, acc: Long): Long = if n == 0 then acc else sum(n - 1, acc + n)

def attempt(f: => Long): String = try f.toString catch case _: Throwable => "overflow"

@main def run(): Unit =
  given Int = 1
  println(attempt(summon[Sum].sum(1000000, 0)))
