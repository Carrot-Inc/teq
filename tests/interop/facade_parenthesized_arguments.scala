// A facade's annotation arguments and a native default written in parentheses: the import is to be
// bound and the default is to be `js.native`, an omitted argument.
import scala.scalajs.js
import scala.scalajs.js.annotation.{JSGlobal, JSImport}

@js.native
@JSImport(("node:path"), ("posix"))
object Path extends js.Object:
  def basename(p: String): String = js.native

object Native:
  @js.native @JSGlobal(("parseInt"))
  def parse(s: String, radix: Int = ((js.native))): Int = js.native

@main def run(): Unit =
  println(Path.basename("/a/b/c.txt"))
  println(Native.parse("12", 10))
  println(Native.parse("ff", 16))
  println(Native.parse("21"))
