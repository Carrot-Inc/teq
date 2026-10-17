// teq: --werror
// `@unchecked` under another annotation still silences what it silences alone: a type test (TypeTestsCasts' replaceP
// maps through the AnnotatedType), and on an expression a match's exhaustivity and reachability and a pattern
// definition's refutability (`hasAnnotation` looks through stacked annotations). scalac 3.8.4 accepts all under -Werror.
class Mark extends scala.annotation.StaticAnnotation

final case class Box[A](a: A)

object Ints:
  def unapply(x: List[Int] @unchecked @Mark): Option[Int] = x.headOption

object Held:
  val Some(m) = Option(8): @unchecked @Mark

object Main:
  def matched(x: Option[Int]): Int = (x: @unchecked @Mark) match
    case Some(n) => n
  def matchedOther(x: Option[Int]): Int = (x: @Mark @unchecked) match
    case Some(n) => n + 1
  def matchedLayers(x: Option[Int]): Int = ((x: @unchecked): @Mark) match
    case Some(n) => n + 2
  def reachable(x: Option[Int]): Int = (x: @unchecked @Mark) match
    case _ => 1
    case Some(n) => n
  def first(x: Any): Int = x match
    case xs: (List[Int] @unchecked @Mark) => xs.size
    case _ => 0
  def second(x: Any): Int = x match
    case xs: (List[Int] @Mark @unchecked) => xs.size + 10
    case _ => 0
  def boxed(x: Any): Int = x match
    case Box(xs: List[Int @unchecked @Mark]) => xs.sum
    case _ => 0
  def nested(x: Any): Int = x match
    case b: Box[List[Int] @unchecked @Mark] => b.a.size + 100
    case _ => 0
  def extracted(x: List[Any]): Int = x match
    case Ints(n) => n
    case _ => -1
  def main(args: Array[String]): Unit =
    println(s"${first(List(1))} ${second(List(1, 2))} ${boxed(Box(List(3, 4)))} ${nested(Box(List(5)))} ${extracted(List(7))}")
    val Some(k) = Option(6): @unchecked @Mark
    println(s"${matched(Some(7))} ${matchedOther(Some(7))} ${matchedLayers(Some(7))} ${reachable(Some(7))} $k ${Held.m}")
