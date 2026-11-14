// expect: Main.scala:12:45: error: forward reference to second (defined on line 9) extends over the definition of held (on line 8)
// expect: inlined from tests/errors/forward_reference_quoted/Macros.scala:7
// expect: Main.scala:13:38: error: forward reference to Later (defined on line 17) extends over the definition of held (on line 16)
// expect: inlined from tests/errors/forward_reference_quoted/Macros.scala:15
// expect: 2 errors found
// absent: Main.scala:14:
// A quote's block is not checked where it is written; its copy is, where the macro is called: at the call's
// point, with the quote's line as the place it was inlined from. A later object of the quote's block is a
// reference there, a creation of a later class of the quote's block is none (scalac's check does not see it).
import Macros.*
object Test:
  def viaMethod(n: Int): Int = Macros.method(n)
  def viaModule(n: Int): Int = module(n)
  def viaClass(n: Int): Int = Macros.created(n)
  def main(args: Array[String]): Unit = println(viaClass(3))
