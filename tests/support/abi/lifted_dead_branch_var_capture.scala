// A var that a lambda reads only in a branch the constant fold drops is not captured: scalac writes the lambda's body
// `$anonfun$1()I` of `Main$`, with no `ObjectRef` or `IntRef` parameter. Every member of every class is listed (no `//
// abi:` line).
object Main {
  def run(n: Int): Int = {
    var x = n
    val f = () => { if (false) x += 1; 7 }
    x += 2
    f() + x
  }
  def main(args: Array[String]): Unit = println(run(args.length))
}
