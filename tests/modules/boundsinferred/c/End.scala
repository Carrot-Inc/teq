package bni
object End:
  def use(c: Ctx) = Mid.forward(c)
  def main(args: Array[String]): Unit = println(Api.toString.nonEmpty)
