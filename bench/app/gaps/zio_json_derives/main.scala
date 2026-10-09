import zio.json.*
case class Shelf(id: Long, name: String) derives JsonCodec
object Main:
  def main(args: Array[String]): Unit =
    println(Shelf(1L, "a").toJson)
