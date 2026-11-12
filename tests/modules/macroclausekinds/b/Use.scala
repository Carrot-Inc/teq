// A downstream of a/ that expands a/'s macros, whose implementations
// the interpreter is to run from a/'s products, built by scalac too
// (over-scalac). A pickled application's clause is its method's, the
// alternative its signed name selects: scalac signs the reflection
// trait's parameter `scala.quoted.Quotes.reflectModule.Printer`,
// and a super accessor's signature selects the alternative
// of the member it calls (SuperAccessors.scala:77-81).
package mcb

@main def run(): Unit =
  println(mck.Macros.Mac.printer)
  println(mck.Macros.Mac.sup)
