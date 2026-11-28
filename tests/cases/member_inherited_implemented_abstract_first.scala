abstract class Subject extends Parent:
  override def pick(x: Int): Int

trait Parent:
  def pick(x: Int): Int = x + 1

class Child extends Subject:
  override def pick(x: Int): Int = x + 1

object Main:
  def main(args: Array[String]): Unit =
    val parent: Parent = new Child
    println(parent.pick(41))
