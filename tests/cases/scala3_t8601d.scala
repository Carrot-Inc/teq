//> using platform jvm
// Copied from scala3 tests/run/t8601d.scala (Apache-2.0, see tests/scala3/README.md).
// scalajs: --compliant-semantics

object Test {
  def monitor(x: AnyRef): Unit = {x.synchronized(()); ()}
  def check(x: => Any) = try { x; sys.error("failed to throw NPE") } catch { case _: NullPointerException => }

  def main(args: Array[String]): Unit = {
    check(monitor(null))
  }
}
