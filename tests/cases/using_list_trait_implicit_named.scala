// A trait's implicit clause applied with a short `using` list of named arguments, a parameter omitted before a written
// one: it is searched as scalac searches it (50), after the written argument.
trait Ts(implicit a: Int, b: Int, c: Int):
  def value: Int = a + b + c
object Lib:
  given Int = 20
  // the named list
  def named: Int = (new Ts(using b = 10) {}).value
object Main:
  def main(args: Array[String]): Unit =
    println(Lib.named)
