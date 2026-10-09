// `&&` and `||` are the short-circuit operators of a Boolean; on any other receiver they are
// members or extension methods like every other operator.
final class Box(val n: Int):
  def &&(that: Box): Box = new Box(n + that.n)
  def ||(that: Box): Box = new Box(n * that.n)
  def +(that: Box): Box = new Box(n + that.n + 100)
  def -(that: Box): Box = new Box(n - that.n)

final case class Sched(steps: List[String]):
  def jittered: Sched = Sched(steps.map(_ + "~"))
object Sched:
  def recurs(n: Int): Sched = Sched(List(s"recurs($n)"))
  def exponential(ms: Int): Sched = Sched(List(s"exp($ms)"))
extension (a: Sched) def &&(b: Sched): Sched = Sched(a.steps ++ b.steps)
extension (a: Sched) def ||(b: Sched): Sched = Sched(List(a.steps.mkString("|") + "/" + b.steps.mkString("|")))

def loud(b: Boolean, tag: String): Boolean =
  println(tag)
  b

@main def main(): Unit =
  println((new Box(1) && new Box(2)).n)
  println((new Box(3) || new Box(2)).n)
  println((new Box(3) + new Box(2)).n)
  println((new Box(3) - new Box(2)).n)
  println(Sched.recurs(3) && Sched.exponential(300).jittered)
  println((Sched.recurs(1) || Sched.recurs(2)) && Sched.exponential(1))
  println(loud(false, "left") && loud(true, "right"))
  println(loud(true, "left") || loud(true, "right"))
  val c = "x".length == 1
  println((if c then true else false) && c)
  val mixed: Boolean | Int = 3
  mixed match
    case b: Boolean => println(b && true)
    case i: Int => println(i > 2 && i < 4)
