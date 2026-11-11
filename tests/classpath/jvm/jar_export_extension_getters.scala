// jars: scala-library defaults-lib
// An object re-exporting a jar's export forwarder made through a companion extension method (tests/support/defaults_lib.scala's
// `OpsExports.f`): the re-export's getter passes the jar's getter its own parameters, the path method's among them
// (`f$default$2(s: String)(a: Int)`, `addForwarder` of the getter, Namer.scala:1385-1387).
import defaultslib.*

object Q {
  export OpsExports.f
}

@main def main(): Unit =
  println(OpsExports.f("xy")(2)(7, 8))
  println(Q.f("xy")(2)(7, 8))
