import zio.json.*
case class Shelf(id: Long, name: String)
object Shelf:
  given JsonEncoder[Shelf] = JsonEncoder[String].contramap(s => s.name)
object Main:
  def main(args: Array[String]): Unit =
    println(Shelf(1L, "a").toJson)
    println("[1,2]".fromJson[List[Int]])
