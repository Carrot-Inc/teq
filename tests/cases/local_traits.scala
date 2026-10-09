// A sealed trait local to a method with case objects and a case class below it, as zio's
// `Cause.foldContext` has; a local trait with a concrete member, mixed into a local class and
// an anonymous one; a local trait over the method's type parameter.
object Main:
  def fold(xs: List[Int]): String =
    sealed trait Step
    case object Both extends Step
    case object Then extends Step
    final case class Stackless(flag: Boolean) extends Step
    def loop(in: List[Int], out: List[Either[Step, String]]): List[String] =
      in match
        case 0 :: rest => loop(rest, Left(Both) :: out)
        case 1 :: rest => loop(rest, Left(Then) :: out)
        case n :: rest if n < 0 => loop(rest, Left(Stackless(n < -5)) :: out)
        case n :: rest => loop(rest, Right(n.toString) :: out)
        case Nil =>
          out.foldLeft[List[String]](Nil) {
            case (acc, Right(s)) => s :: acc
            case (acc, Left(Both)) => ("both" + acc.size) :: acc
            case (acc, Left(Then)) => "then" :: acc
            case (acc, Left(Stackless(f))) => s"stackless($f)" :: acc
          }
    loop(xs, Nil).mkString(",")

  def greet(names: List[String]): List[String] =
    trait Greeter:
      def prefix: String
      def greet(n: String): String = prefix + ", " + n
    class Hello extends Greeter:
      def prefix = "hello"
    val hi = new Greeter:
      def prefix = "hi"
    names.map(n => if n.length > 3 then Hello().greet(n) else hi.greet(n))

  def boxes[A](xs: List[A]): List[String] =
    trait Box:
      def value: A
      override def toString = s"Box($value)"
    xs.map(x => new Box { def value = x }.toString)

  def main(args: Array[String]): Unit =
    println(fold(List(3, 0, 1, -7, -2, 9)))
    println(greet(List("Ann", "Robert")))
    println(boxes(List(1, 2)))
    println(Stackless0.check)

object Stackless0:
  def check: Boolean =
    sealed trait K
    case object A extends K
    val k: K = A
    k match
      case A => true
