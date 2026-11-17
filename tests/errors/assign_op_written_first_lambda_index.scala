// expect: 20:5: error: Ambiguous overload. The overloaded alternatives of method apply in class C with types
// expect: 1 error found
// Where both tries of an assignment operator's application fail, the first failure alone is told: the function
// literal the first try typed as an index does not reach the second try, whose own errors (the index typed by
// itself) go with it. scalac tells one line too, another one: it takes the alternative of `apply` whose result
// has the operator and tells the argument's mismatch (E007: found 2, required String); teq's first try stops at
// the alternatives, which it does not yet choose by the member selected on the result.
import scala.annotation.targetName
class A
class B:
  def +=(s: String): Unit = println("member")
  def +(n: Int): B = { println("plus"); this }
class C:
  def apply(f: Int => Int): A = { println("int"); new A }
  @targetName("applyString") def apply(f: String => String): B = { println("string:" + f("s")); new B }
  def update(f: String => String, b: B): Unit = println("update")
object Test:
  def main(args: Array[String]): Unit =
    val c = new C
    c(x => x + 1) += 2
