// An argument whose type is an intersection of two applications of one class, sixty levels of arguments deep, where
// a type variable is expected: before the variable is constrained the intersection is compared with the variable's
// lower bound, Nothing, and the two applications' arguments are compared with each other on the way.
trait A
trait B
trait F[+T]

object Main:
  def id[T](t: T): T = t
  def pass(x: F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[A]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]] & F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[B]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]): F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[A]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]] & F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[F[B]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]] = id(x)
  def main(args: Array[String]): Unit =
    println("passed")
