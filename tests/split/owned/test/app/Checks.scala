package app

object Checks:
  def main(args: Array[String]): Unit =
    println(s"action ${Actions.make().run()}")
    println(Discounted("pad", 4).describe)
    println(s"total ${Catalog.total}")

class Discounted(name: String, price: Int) extends Item(name, price):
  override def describe: String = s"discounted ${super.describe}"
