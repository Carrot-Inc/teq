// A trait's implicit clause applied with a short `using` list, by an anonymous class and by a class's parent, keeps
// teq's refusal (scalac takes the given for the rest: 30, 30, 50): a body kept in lean products loses such a list's
// written arguments, a given of their type in scope taking their place, as master's whole list does, so the search
// stays out until that print keeps the list (a part of its own).
// expect: 15:24: error: missing argument for parameter b
// expect: 16:19: error: missing argument for parameter b
// expect: 18:25: error: missing argument for parameter a
// expect: 18:25: error: missing argument for parameter c
trait Tr(implicit a: Int, b: Int):
  def value: Int = a + b
trait Ts(implicit a: Int, b: Int, c: Int):
  def value: Int = a + b + c
object Lib:
  given Int = 20
  def anon: Int = (new Tr(using 10) {}).value
  class C extends Tr(using 10)
  def parent: Int = (new C).value
  def named: Int = (new Ts(using b = 10) {}).value
object Main:
  def main(args: Array[String]): Unit =
    println(Lib.anon)
    println(Lib.parent)
    println(Lib.named)
