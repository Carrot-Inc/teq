// An enum case of two parameters, both the lambda's and one of them.
enum Node:
  case Two(a: Int, b: String)
  case Plain
import Node.*
def on[A, B](f: A => B): A => B = f
def on2[A, B, C](f: (A, B) => C): (A, B) => C = f
@main def run(): Unit =
  val f = on2(Two(_, _))
  val g = on(Two(_, "s"))
  println(s"${f(1, "t")} ${g(2)}")
