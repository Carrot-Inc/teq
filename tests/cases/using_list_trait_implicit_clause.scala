// A trait's implicit clause applied with a short `using` list, by an anonymous class and by a class's parent: the
// rest are searched as scalac searches them (30, 30).
trait Tr(implicit a: Int, b: Int):
  def value: Int = a + b
object Lib:
  given Int = 20
  def anon: Int = (new Tr(using 10) {}).value
  class C extends Tr(using 10)
  def parent: Int = (new C).value
object Main:
  def main(args: Array[String]): Unit =
    println(Lib.anon)
    println(Lib.parent)
