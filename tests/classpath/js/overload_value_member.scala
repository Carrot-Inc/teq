// jars: fixtures
// targets: js interp
// A jar trait's overloaded parameterless member implemented by a case class field, called from
// the trait's own body (tapir's `EndpointInfoOps.tag` over `Endpoint.info`), and a field beside
// an inherited method of its name that takes parameters.
import fix.overloads.{Eps, OvBox}

@main def main(): Unit =
  val e = Eps.start.tag("b")
  println(e)
  println(e.show)
  println(e.info("c").tag("d").info)
  println(OvBox(3).size)
  println(OvBox(3).describe)
