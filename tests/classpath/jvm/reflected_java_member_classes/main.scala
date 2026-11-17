class Rack extends nest.Shelf[String] { type Label = Int }

object Main {
  def main(args: Array[String]): Unit = {
    Lists.of[nest.Shelf[String]].foreach(println)
    Lists.of[Rack].foreach(println)
  }
}
