final class Turn(val id: Int) {
  def play(a: Int, b: Int, round: Int): Int = {
    println(s"$id $a $b $round")
    if round < 2 then new Turn(id + a).play(b, a, round + 1)
    else id + a - b
  }
}
object Main {
  def main(args: Array[String]): Unit = println(new Turn(1).play(4, 7, 0))
}
