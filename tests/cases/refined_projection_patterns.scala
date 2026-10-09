// A projection on an alias of a refinement (`Zippable.Out[A, B, C]#Out`), sequence patterns over a
// case class's repeated parameter, and a self type's methods overloading the class's own.
trait Zippable[-A, -B]:
  type Out
  def zip(a: A, b: B): Out
object Zippable:
  type Out[-A, -B, O] = Zippable[A, B] { type Out = O }
  given pair[A, B]: Zippable.Out[A, B, (A, B)] = new Zippable[A, B]:
    type Out = (A, B)
    def zip(a: A, b: B): Out = (a, b)
def zipAll[A, B](as: List[A], bs: List[B])(using z: Zippable[A, B]): List[z.Out] = as.zip(bs).map((a, b) => z.zip(a, b))
def first(p: Zippable.Out[Int, String, (Int, String)]#Out): Int = p._1

final case class Al(alias: String, aliases: String*)

trait Ops[A]:
  def toArray[B >: A]: List[B] = Nil
sealed abstract class Ch[A] extends Ops[A]:
  protected def toArray[A1 >: A](srcPos: Int, dest: Array[A1], destPos: Int, length: Int): Unit =
    dest(destPos) = value
  def value: A
trait ChLike[A] extends Ops[A] { self: Ch[A] =>
  def copyToArray[B >: A](dest: Array[B], destPos: Int, length: Int): Int =
    toArray[B](0, dest, destPos, length)
    length
}
final class One[A](val value: A) extends Ch[A] with ChLike[A]

object Main:
  def main(args: Array[String]): Unit =
    val zs = zipAll(List(1, 2), List("a", "b"))
    println(zs.map { case (l, r) => l.toString + r })
    println(first((3, "c")))
    val x: Any = Al("a", "b", "c")
    x match
      case Al(a, rest*) => println(a + " " + rest.toList)
    x match
      case Al(a, _*) => println(a)
    x match
      case Al(a, b, c) => println(a + b + c)
      case _ => println("other")
    x match
      case Al(a, b) => println("two")
      case Al(a, b, rest*) => println(a + b + " " + rest.length)
    val dest = Array(0, 0)
    println(new One(7).copyToArray(dest, 1, 1) + " " + dest.toList)
