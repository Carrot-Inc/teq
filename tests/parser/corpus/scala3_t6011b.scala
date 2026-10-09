// Adapted from scala3 tests/run/t6011b.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
object Test {
  def main(args: Array[String]): Unit = ()
  var cond = true

  // should not generate a switch
  def f(ch: Char): Int = ch match {
    case 'a' if cond => 1
    case 'z' | 'a' => 2
  }

  println(f('a') + f('z')) // 3
}
