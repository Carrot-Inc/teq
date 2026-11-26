// A local def and a lambda of a class's method that read a field only in branches the constant fold drops capture
// nothing: scalac writes both as static methods of `Box`, `deadThis$1()I` and `$anonfun$1()I`, with no `Box` parameter.
// Every member of every class is listed (no `// abi:` line).
class Box(val n: Int) {
  def run(): Int = {
    def deadThis(): Int = if (false) n else 7
    val f = () => if (false) n else 8
    deadThis() + f()
  }
}
object Main { def main(args: Array[String]): Unit = println(new Box(3).run()) }
