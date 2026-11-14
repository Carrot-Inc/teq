package shapes
object Main {
  def check(x: Joined): Unit = { (x: Runnable).run(); (x: Own).run() }
  def main(args: Array[String]): Unit = {
    check(new Both)
    check(new Leaf)
    check(new Joined { def run(): Unit = println("anon") })
    check(Single)
    check(Kind.One)
    check(Record(1))
  }
}
