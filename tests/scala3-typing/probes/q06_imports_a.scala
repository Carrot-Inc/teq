object Q:
  val a = 1
  val b = 2
import Q.{a, a}
@main def run(): Unit = println(a)
