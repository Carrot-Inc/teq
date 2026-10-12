//> using dep com.softwaremill.macwire::macros:2.6.7
// jars: macwire-macros
import com.softwaremill.macwire.*

object Env:
  val coll = new Coll
  given s: String = "x"
  lazy val api = wire[Api]

@main def main() = println(Env.api.show)
