// A block's inline method first needed in the default of another inline method's parameter, or in an argument of a
// call of a dependent method that recurses, is the method scalac types where it stands in the block: a function
// nested in its body is called and returns (42), and a call in its body of a method whose parameter's type names the
// enclosing method's parameter takes that parameter's path, whatever the recursive call's own argument is
// (`end/2/2`).
trait Sized:
  type T
  val value: T

object IntSized extends Sized:
  type T = Int
  val value = 2

object StrSized extends Sized:
  type T = String
  val value = "str"

object Needed:
  def defaults(): Int =
    inline def caller(x: Int = helper): Int = x
    inline def helper: Int =
      def ordinary(): Int = 42
      ordinary()
    caller()
  def paths(b: Sized)(a: b.T, again: Boolean): String =
    def own(x: b.T): String = x.toString
    val first = if again then paths(StrSized)(helper, false) else "end"
    inline def helper: String = own(a)
    first + "/" + own(a)

@main def run(): Unit =
  println(Needed.defaults())
  println(Needed.paths(IntSized)(2, true))
