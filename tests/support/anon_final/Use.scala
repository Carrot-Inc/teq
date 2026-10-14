package anonfinal

object Use:
  val id: QuotedId[Int] = Mac.quoted
  def main(args: Array[String]): Unit = println(id.name + id.value(id.wrap("abc")) + Mac.plain.name)
