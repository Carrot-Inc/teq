import stall.Tool

// The second module's object extends the first's trait, over the first's products: the trait's body and block
// run with the object's.
object Yard:
  object Main extends Tool:
    println("main body")
    def show(): Unit = println("count " + count)

object Driver:
  def go(): Unit =
    Yard.Main.main(Array("x", "y"))
    Yard.Main.show()

@main def start(): Unit = Driver.go()
