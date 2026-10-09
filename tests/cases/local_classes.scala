// Named local classes: lifted with what they capture, as anonymous classes are.
trait Show:
  def show: String

abstract class Base(val n: Int):
  def describe: String = "base " + n

class Outer(prefix: String):
  def make(k: Int): Show =
    class Tagged(x: Int) extends Base(x + 1) with Show:
      def show: String = prefix + ":" + n + "/" + k
    new Tagged(k * 10)

object Main:
  def counter(): () => Int =
    var count = 0
    class Counter:
      def tick(): Int =
        count += 1
        count
    val c = new Counter
    () => c.tick()

  def sum(xs: List[Int]): Int =
    class Acc extends (Int => Unit):
      var total = 0
      def apply(x: Int): Unit = total += x
    val acc = new Acc
    xs.foreach(acc)
    acc.total

  def sameName(flag: Boolean): String =
    if flag then
      class A(x: Int):
        override def toString: String = "first A(" + x + ")"
      new A(1).toString
    else
      class A(s: String):
        override def toString: String = "second A(" + s + ")"
      new A("z").toString

  def linked(): Int =
    class Node(val value: Int, val next: Option[Node]):
      def total: Int = value + next.map(_.total).getOrElse(0)
      def push(v: Int): Node = new Node(v, Some(this))
    new Node(1, None).push(2).push(3).total

  def defaults(): String =
    class D(x: Int = 5, y: String = "y"):
      def text: String = x.toString + y
    new D().text + " " + new D(1).text + " " + new D(2, "b").text

  def typed(): String =
    class P(val a: Int)
    val items: List[Any] = List(new P(1), "s", new P(2))
    items.collect { case p: P => p.a }.mkString(",")

  def main(args: Array[String]): Unit =
    println(new Outer("p").make(3).show)
    val tick = counter()
    tick(); tick()
    println(tick())
    println(sum(List(1, 2, 3, 4)))
    println(sameName(true))
    println(sameName(false))
    println(linked())
    println(defaults())
    println(typed())
    val f = (n: Int) =>
      class Sq(v: Int):
        def value: Int = v * v + n
      new Sq(n).value
    println(f(4))
