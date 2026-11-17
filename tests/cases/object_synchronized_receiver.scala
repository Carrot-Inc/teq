// The receiver of `synchronized` is evaluated: an object named there is initialised first, whether its
// initialisation is its own statement or a superclass's body.
class Loud(tag: String) { println("  body of " + tag) }
object Inherited extends Loud("Inherited")
object Own { println("  Own's statement") }
object Quiet
object Main {
  def main(args: Array[String]): Unit = {
    println("inherited"); Inherited.synchronized { println("  in") }
    println("own"); Own.synchronized { println("  in") }
    println("quiet"); Quiet.synchronized { println("  in") }
    println("end")
  }
}
