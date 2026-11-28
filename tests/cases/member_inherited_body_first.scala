class Subject extends Parent:
  def pick(x: Int): Int = x + 1

trait Parent:
  def pick(x: Int): Int

object Main:
  def main(args: Array[String]): Unit =
    val parent: Parent = new Subject
    println(parent.pick(41))
