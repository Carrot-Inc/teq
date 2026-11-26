// A block that ends in a class literal is kept by the constant fold (dotty's `TreeInfo.constToLiteral` keeps every
// block whose result is a class literal), its definitions with it: scalac writes the local class `Main$Local$1` and the
// local def `kept$1` of `Main$`. Every member of every class is listed (no `// abi:` line).
object Main {
  def run(): Class[?] = { class Local; def kept(): Int = 7; classOf[Local] }
  def main(args: Array[String]): Unit = println(run() != null)
}
