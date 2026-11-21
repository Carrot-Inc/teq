// A trait's implicit clause applied with a short `using` list of named arguments, a parameter omitted before a written
// one: teq keeps the refusal for it, where scalac takes the given for it (50), since teq's lifting of a trait parent's
// arguments out of their order would place it after the written one (`limit_using_search`); a trailing one is
// searched. A part of its own.
// expect: 11:25: error: missing argument for parameter a
trait Ts(implicit a: Int, b: Int, c: Int):
  def value: Int = a + b + c
object Lib:
  given Int = 20
  // the named list
  def named: Int = (new Ts(using b = 10) {}).value
object Main:
  def main(args: Array[String]): Unit =
    println(Lib.named)
