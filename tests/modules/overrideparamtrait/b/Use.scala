import lib.*
import scala.annotation.nowarn
@nowarn class UC(override val v: String) extends B
class UD extends UC("uc-arg") with TR
@main def run(): Unit =
  println(new UD().read + " " + new UC("uc").read)
