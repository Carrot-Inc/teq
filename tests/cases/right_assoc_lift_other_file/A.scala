// A top-level value of another file is no receiver whose evaluation does nothing: its first read runs that file's
// initializers, so the operand is bound ahead of it. Master read the value first.
import other.*
object Main:
  def left(): Int = { println("left"); 0 }
  def main(args: Array[String]): Unit =
    println("start")
    println(left() :: items)
    println(left() :: Holder.held)
