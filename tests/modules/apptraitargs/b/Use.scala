// The second module's object extends the first's trait and so `App`, over the first's products.
object Yard:
  object Main extends stall.Tool:
    println("main body")

object Use:
  def main(a: Array[String]): Unit =
    Yard.Main.main(Array("x", "y"))
    println("count " + Yard.Main.count)
