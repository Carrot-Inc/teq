package r2nullreturn

object Main:
  def main(args: Array[String]): Unit =
    try println(Api.twice(new Empty))
    catch case _: Throwable => println("nested failed")
    try println(Api.arrayString(null))
    catch case _: Throwable => println("array failed")
    println(Api.result(new Empty))
    val x: Any = new Empty
    println(x.toString == null)
