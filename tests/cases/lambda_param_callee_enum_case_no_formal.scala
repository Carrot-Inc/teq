// An enum case applied to the lambda's parameter where no type is expected of the lambda.
enum Node:
  case Wrapped(inner: Int)
  case Plain
import Node.*
@main def run(): Unit =
  val f = Wrapped(_)
  println(f(1))
