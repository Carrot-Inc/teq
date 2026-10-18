//> using platform jvm
// Object.wait's receiver, then its arguments in the order written, out of the parameters' order, then the null
// receiver's NullPointerException, as scalac's application evaluates them.
object Main {
  def recv(): Object = { println("recv"); null }
  def nanos(): Int = { println("nanos"); 0 }
  def millis(): Long = { println("millis"); 1L }
  def main(args: Array[String]): Unit = {
    try recv().wait(x$1 = nanos(), x$0 = millis())
    catch { case _: NullPointerException => println("null") }
  }
}
