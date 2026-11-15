import other.*
object Main:
  def left(): Int = { println("left"); 0 }
  def main(args: Array[String]): Unit =
    println("start")
    println(left() :: items)
    println(left() :: Holder.held)
    println(left() :: Quiet.held)
    println(left() +: Op)
    println(left() +: Param)
