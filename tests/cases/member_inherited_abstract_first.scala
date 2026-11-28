trait Parent:
  def pick(x: Int): Int

class Subject extends Parent:
  def pick(x: Int): Int = x + 1

object Main:
  def main(args: Array[String]): Unit =
    val parent: Parent = new Subject
    println(parent.pick(41))
