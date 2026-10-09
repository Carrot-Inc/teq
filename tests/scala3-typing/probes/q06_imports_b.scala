object Q:
  val a = 1
  val b = 2
import Q.{*, b}
@main def run(): Unit = println(a)
