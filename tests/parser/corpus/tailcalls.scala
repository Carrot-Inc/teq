import scala.annotation.tailrec
import scala.util.Try

object Rec:
  @tailrec def loop(n: Int, acc: Long): Long = if n == 0 then acc else loop(n - 1, acc + n)
  def loopNoAnnot(n: Int, acc: Long): Long = if n == 0 then acc else loopNoAnnot(n - 1, acc + n)
  def evenOdd(n: Int): Boolean = if n == 0 then true else odd(n - 1)
  def odd(n: Int): Boolean = if n == 0 then false else evenOdd(n - 1)
  def local(n: Int): Int =
    @tailrec def go(i: Int, acc: Int): Int = if i == 0 then acc else go(i - 1, acc + 1)
    go(n, 0)
  def tailMatch(xs: List[Int], acc: Int): Int = xs match
    case Nil => acc
    case h :: t => tailMatch(t, acc + h)
  def tailIf(n: Int, acc: Int): Int =
    if n <= 0 then acc
    else if n % 2 == 0 then tailIf(n - 1, acc + 2)
    else tailIf(n - 1, acc + 1)
  final def finalLoop(n: Int): Int = if n == 0 then 0 else finalLoop(n - 1)
  private def privLoop(n: Int): Int = if n == 0 then 0 else privLoop(n - 1)
  def callPriv(n: Int) = privLoop(n)
  def unit(n: Int): Unit = if n > 0 then unit(n - 1)
  def tick(i: Int): Unit = if i < 0 then () else tick(i - 1)
  def defaults(i: Int = 0, acc: Int = 100): Int = if i >= 3 then acc else defaults(i + 1)
  def secondList(n: Int)(m: Int = n * 2): Int = if n == 0 then m else secondList(n - 1)()
  def closures(n: Int, acc: List[() => Int]): List[() => Int] =
    if n == 0 then acc else closures(n - 1, (() => n) :: acc)
  def blockBody(n: Int, acc: Int): Int =
    val next = n - 1
    if n == 0 then acc
    else
      val step = acc + 1
      blockBody(next, step)

sealed trait BK
case object Empty extends BK
final case class Tree(v: Int, children: Map[Int, BK]) extends BK:
  @tailrec def has(a: Int): Boolean = this match
    case Tree(v, cs) =>
      v == a || (cs.get(1) match
        case Some(w: Tree) => w.has(a)
        case _ => false)
  def none(a: Int): Boolean = v != a && (children.get(1) match
    case Some(w: Tree) => w.none(a)
    case _ => true)

final class Node(val v: Int, val next: Option[Node]):
  def last: Int = next match
    case Some(n) => n.last
    case None => v
  @tailrec def nth(i: Int): Int = if i == 0 then v else next.get.nth(i - 1)
  def fs(i: Int, acc: List[() => Int]): List[() => Int] =
    if i == 0 then acc else next.get.fs(i - 1, (() => this.v) :: acc)

@main def main(): Unit =
  val n = 1000000
  println("tailrec  " + Try(Rec.loop(n, 0)))
  println("noannot  " + Try(Rec.loopNoAnnot(n, 0)))
  println("local    " + Try(Rec.local(n)))
  println("match    " + Try(Rec.tailMatch(List.fill(n)(1), 0)))
  println("if-chain " + Try(Rec.tailIf(n, 0)))
  println("final    " + Try(Rec.finalLoop(n)))
  println("private  " + Try(Rec.callPriv(n)))
  println("mutual5k " + Try(Rec.evenOdd(5000)))
  Rec.unit(n)
  Rec.tick(n)
  println("defaults " + Rec.defaults() + " " + Rec.secondList(3)())
  println("closures " + Rec.closures(3, Nil).map(_()))
  println("block    " + Rec.blockBody(n, 0))
  val chain = (1 to 5).foldRight(Option.empty[Node])((i, next) => Some(Node(i, next))).get
  println("receiver " + chain.last + " " + chain.nth(3) + " " + chain.fs(4, Nil).map(_()))
  val deep = (1 to 100000).foldLeft(Empty: BK)((acc, i) => Tree(i, Map(1 -> acc))).asInstanceOf[Tree]
  println("operators " + deep.has(1) + " " + deep.has(-1) + " " + deep.none(-1) + " " + deep.none(5))
