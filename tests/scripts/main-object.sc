// A script whose only content is an object with a main method: the wrapper calls that main with
// the script's arguments.
object Hello:
  def main(args: Array[String]): Unit = println("main of Hello " + args.toList)
