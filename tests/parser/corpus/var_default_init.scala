// `var x: T = _` starts a field at the zero of its type, as `uninitialized` does.
import scala.compiletime.uninitialized

class Cell:
  var value: String = _
  var count: Int = _
  var flag: Boolean = _
  var items: List[Int] = uninitialized
  def bump(): Unit = count += 1

object Main:
  def main(args: Array[String]): Unit =
    val c = new Cell
    println(s"${c.value} ${c.count} ${c.flag} ${c.items}")
    c.bump()
    c.value = "v"
    println(s"${c.value} ${c.count}")
