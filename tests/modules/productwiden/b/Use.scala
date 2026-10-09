package widenuse
import widenapi.*
object Main:
  def main(args: Array[String]): Unit =
    val i = List(3).head
    val b: Byte = 4
    val s: Short = 5
    println(Api.wide(i) + 0.25)
    println(Api.short(b) + Api.integer(s))
    println(Api.default() + 0.25)
    val t = Api.tuple(i, b, s)
    println(t._1 + t._2 + t._3 + 0.25)
    val c = Api.box(i, b, s)
    println(c.d + c.s + c.i + 0.25)
    println(Api.wideChar('A') + 0.25)
    println((Api.floatChar('B') * 10).toInt)
    println(Api.rounded(16777217 + i - 3).toLong)
