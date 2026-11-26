// An inline def's lambda applied at the call is reduced to its body once inlined (dotty's `BetaReduce`): scalac writes
// no lambda method. Every member of every class is listed (no `// abi:` line).
object Main {
  inline def make = (x: Int) => x + 1
  def run(n: Int): Int = make(n)
  def main(args: Array[String]): Unit = println(run(args.length))
}
