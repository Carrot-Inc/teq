import scala.idontexist
import demo.implicits.*
object Q:
  val a = 1
  val b = 2
object R:
  val a = 3
import Q.{a, a}
import Q.{*, b}
object Test:
  import Q.a
  import R.a
  def v = a
@main def run(): Unit = println(Test.v)
