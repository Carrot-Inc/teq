case class Bag(items: List[Int])

extension (bag: Bag)
  def find(p: Int => Boolean): Option[Int] = bag.items.find(p)
  def exists(p: Int => Boolean): Boolean = find(p).isDefined
  def firstEven: Option[Int] = find(_ % 2 == 0)
  def total: Int = bag.items.sum
  def average: Int = if count == 0 then 0 else total / count
  def count: Int = bag.items.length
  def describe(find: String): String = find + total
  def countdown(n: Int): List[Int] = if n <= 0 then Nil else n :: countdown(n - 1)
  def nested: Int =
    def helper(k: Int): Int = k + total
    List(1, 2).map(n => helper(n) + count).sum

extension [A](xs: List[A])
  def second: Option[A] = xs.drop(1).headOption
  def secondOr(default: A): A = second.getOrElse(default)
  def pairUp[B](ys: List[B]): List[(A, B)] = xs.zip(ys)
  def pairedWithSelf: List[(A, A)] = pairUp(xs)

object Ops:
  def label: String = "ops"
  extension (n: Int)
    def double: Int = n * 2
    def quadruple: Int = double.double
    def labelled: String = label + quadruple

def total: Int = -1

@main def main(): Unit =
  val bag = Bag(List(1, 2, 3, 4))
  println(bag.exists(_ > 3))
  println(bag.exists(_ > 4))
  println(bag.firstEven)
  println(bag.average)
  println(bag.describe("sum "))
  println(bag.countdown(3))
  println(bag.nested)
  println(List(1, 2, 3).secondOr(0))
  println(List("a").secondOr("none"))
  println(List(1, 2).pairedWithSelf)
  import Ops.*
  println(3.labelled)
  println(total)
