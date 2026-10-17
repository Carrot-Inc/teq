package app

trait Show[A] { def show(a: A): String }

object Main {
  given Show[Int] with { def show(a: Int): String = s"shown $a" }
  def main(args: Array[String]): Unit = println(summon[Show[Int]].show(lib.Util.value + lib.Util.`given` + lib.Util.`enum`(0)))
}
