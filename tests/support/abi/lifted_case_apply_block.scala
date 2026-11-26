// A block of a case class's synthetic `apply` and a local def that ends in a literal folds to the literal (dotty's
// `TreeInfo.isPureApply` takes `isPureSyntheticCaseApply`): scalac writes no local def `gone$1`. Every member of every
// class is listed (no `// abi:` line).
case class Value(n: Int)
object Main {
  def run(n: Int): Int = { val v = Value(n); def gone(): Int = 9; 2 }
  def main(args: Array[String]): Unit = println(run(args.length))
}
