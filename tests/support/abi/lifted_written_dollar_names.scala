// Written names that read as lifted ones: the method `api$1` is a public member of `Main$` with its static forwarder in
// `Main`, and the local def `f$default$1` is lifted as `f$default$1$1`. Every member of every class is listed (no `//
// abi:` line).
object Main {
  def `api$1`(): Int = 4
  def run(): Int = { def `f$default$1`(): Int = 5; `f$default$1`() }
  def main(args: Array[String]): Unit = println(`api$1`() + run())
}
