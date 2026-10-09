// jars: scala-library abi-callbacks-lib
// std: scala-library
// An enum's values are public static fields of its companion, as scalac writes them and a jar's
// bytecode reads them, for a companion the backend writes and a written one.
import abi.ReflectModule
enum Col:
  case R, G
  case Mix(n: Int)
enum Dir:
  case Up, Down
object Dir:
  def flip(d: Dir): Dir = if d == Up then Down else Up
@main def run(): Unit =
  println(ReflectModule.staticField("Col$", "G"))
  println(ReflectModule.staticField("Dir$", "Down"))
  println(ReflectModule.staticField("Dir$", "Up") == Dir.Up)
  println(Dir.flip(Dir.Up))
