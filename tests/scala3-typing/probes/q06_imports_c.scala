object Q:
  val a = 1
object R:
  val a = 3
object Test:
  import Q.a
  import R.a
  def v = a
@main def run(): Unit = println(Test.v)
