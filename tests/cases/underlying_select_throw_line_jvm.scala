//> using platform jvm
// The line a failure reports where its message is a value class's own parameter selected on the line after the
// value: the line of the `throw`. The JVM's platform: a stack frame's line number.
class V(val text: String) extends AnyVal

object Main:
  def make(s: String): V = new V(s)

  def fail(): Unit =
    throw new RuntimeException(make("failure")
      .text)

  def main(args: Array[String]): Unit =
    println("value=" + make("ok")
      .text)
    try fail()
    catch
      case e: RuntimeException =>
        println(e.getMessage)
        val frames = e.getStackTrace
        println(frames(0).getLineNumber)
