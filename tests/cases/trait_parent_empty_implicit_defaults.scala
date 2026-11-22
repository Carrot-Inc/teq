// An empty list given to a trait's implicit clause is matched as the reference matches it (Applications' matchArgs):
// every parameter takes its default, and no search fills it, by a class parent and an anonymous class; also an implicit
// clause after a using one. No given of the parameters' types is in scope (with one, scalac warns that the default
// shadows it: tests/errors/trait_parent_default_shadows_given).
class A(val s: String)
trait TD(implicit a: Int = 44):
  def value: Int = a
trait TD2(implicit a: Int = 45, b: String = "d"):
  def value: String = a.toString + b
trait TU(using a: A)(implicit n: Int = 46):
  def value: String = a.s + n
class S1 extends TD()()
class S2 extends TD2()()
class S3 extends TU(using A("wa"))()
object Main:
  def main(args: Array[String]): Unit =
    println(new S1().value)
    println((new TD()() {}).value)
    println(new S2().value)
    println((new TD2()() {}).value)
    println(new S3().value)
    println((new TU(using A("wb"))() {}).value)
