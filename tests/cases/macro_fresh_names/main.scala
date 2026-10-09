package fresh

object Main:
  def main(args: Array[String]): Unit =
    println(Stats.summary("alpha", "Alpha", "beta-1", "beta-2"))
    println(Stats.summary("one", "two", "three-x-y"))
    println(Stats.summary("solo"))
