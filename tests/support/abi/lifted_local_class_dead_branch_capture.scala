// A local class whose method reads an enclosing parameter only in a branch the constant fold drops captures nothing:
// scalac writes `Main$Local$1` with a constructor of no parameters and no field. Every member of every class is listed
// (no `// abi:` line).
object Main {
  def run(n: Int): Int = {
    class Local { def value(): Int = if (false) n else 7 }
    new Local().value()
  }
  def main(args: Array[String]): Unit = println(run(args.length))
}
