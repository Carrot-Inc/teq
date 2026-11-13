// jars: rounddemand-lib
// `ValueOf` and the singleton type test of paths through a library's export: the forwarders are the library's
// methods, applied on the exporting object, which the first of them initialises.
package paths

import rounddemand.{Counter, Stock}

def kind(x: Any): String = x match
  case _: Counter.plain.type => "plain"
  case _: Counter.Inner.type => "inner"
  case _ => "none"

@main def run(): Unit =
  println("start")
  println(valueOf[Counter.plain.type])
  println(valueOf[Counter.Inner.type].n)
  println(s"${kind("p")} ${kind(Stock.Inner)} ${kind(1)}")
