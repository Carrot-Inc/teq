// Adapted from scala3 tests/run/exoticnames.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
// this is a run-test because the compiler should emit bytecode that'll pass the JVM's verifier
object Test {
  def main(args: Array[String]): Unit = ()
  def `(` = sys.error("bla")
  def `.` = sys.error("bla")
  def `)` = sys.error("bla")
  def `,` = sys.error("bla")
}
