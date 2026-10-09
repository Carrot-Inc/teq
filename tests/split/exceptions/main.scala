package app

import stock.*
import shop.*

@main def run(): Unit =
  val w = Warehouse(5)
  for n <- List(2, 0, 9) do
    val outcome =
      try order(w, "pen", n)
      catch
        case e: OutOfStock => "out: " + e.item
        case e: StockError => "stock: " + e.getMessage
    println(outcome)
  println(scala.util.Try(w.take(7)).failed.map(_.getMessage))
