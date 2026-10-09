package app

object Main:
  def main(args: Array[String]): Unit =
    Catalog.items.foreach(i => println(i.describe))
    println(Catalog.total)
