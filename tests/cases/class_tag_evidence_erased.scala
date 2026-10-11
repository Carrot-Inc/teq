// The `ClassTag` evidence of the std's array-making members is erased on JavaScript and in the
// interpreter (`@jvmEvidence`): a tag passed to `Array.fill` is typed and not evaluated there, nor
// is a program's tag asked for its array; a program's own evidence parameter is evaluated and
// dispatched on every target. tests/lean-expected holds the lean targets' output.
import scala.reflect.ClassTag

def tag: ClassTag[Int] = { println("evidence"); ClassTag.Int }
class Named extends ClassTag[String]:
  def runtimeClass: Class[?] = classOf[String]
  override def newArray(n: Int): Array[String] = { println(s"newArray $n"); Array.fill(n)("new") }
def make[T](using t: ClassTag[T]): Array[T] = t.newArray(1)

@main def run(): Unit =
  println(Array.fill[Int](1)(2)(using tag).toList)
  println(make[Int](using tag).toList)
  println(Array.fill[String](2)("x")(using new Named).toList)
  println(make[String](using new Named).toList)
