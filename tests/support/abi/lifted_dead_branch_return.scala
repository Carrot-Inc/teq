// A lambda whose `return` stands only in a branch the constant fold drops makes no non-local return: scalac writes the
// lambda's body `$anonfun$1()I` of `Main$`, passed no return key, and `run` catches no `NonLocalReturnControl`. Every
// member of every class is listed (no `// abi:` line).
object Main {
  def run(): Int = {
    val f = () => { if (false) return 3; 7 }
    f()
  }
  def main(args: Array[String]): Unit = println(run())
}
