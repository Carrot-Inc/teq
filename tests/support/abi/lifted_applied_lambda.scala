// A lambda applied where it is written is reduced to its body (dotty's `BetaReduce`), its argument evaluated once:
// scalac writes no lambda method. Every member of every class is listed (no `// abi:` line).
object Main {
  def arg(): Int = { println("arg"); 3 }
  def run(): Int = ((x: Int) => x + 1)(arg())
  def main(args: Array[String]): Unit = println(run())
}
