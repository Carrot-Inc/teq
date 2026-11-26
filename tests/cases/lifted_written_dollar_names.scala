// Written names that read as lifted ones (`api$1`, a local `f$default$1`) are called as written (its class files:
// tests/support/abi/lifted_written_dollar_names.scala).
object Main {
  def `api$1`(): Int = 4
  def run(): Int = { def `f$default$1`(): Int = 5; `f$default$1`() }
  def main(args: Array[String]): Unit = println(`api$1`() + run())
}
