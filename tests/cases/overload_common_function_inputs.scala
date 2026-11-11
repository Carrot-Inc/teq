import scala.annotation.targetName
object CommonFunctionInputs:
  def f(a: Int, b: Int => Int): Int = b(10)
  def f(a: Boolean, b: Int => Int): Int = b(20)
  def id[A](a: A): A = a
  def zero(f: () => Int): String = "int"
  @targetName("zeroText") def zero(f: () => String): String = "string"

@main def runCommonFunctionInputs(): Unit =
  println(CommonFunctionInputs.f(true, CommonFunctionInputs.id))
  println(CommonFunctionInputs.f(1, CommonFunctionInputs.id))
  println(CommonFunctionInputs.zero(() => 1))
  println(CommonFunctionInputs.zero(() => "a"))
