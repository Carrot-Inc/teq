object Q:
  val a = "Q"
object R:
  val a = "R"
object Test:
  import Q.*
  import R.*
  def v = a
object Test2:
  import Q.a
  import R.a
  def v = a
@main def run(): Unit =
  println(Test.v)
  println(Test2.v)
