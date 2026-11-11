// A downstream of a/ that expands a/'s macros, whose implementations the interpreter runs from a/'s products, built
// by scalac too (over-scalac). A pickled application's clause is its method's, the alternative its signed name
// selects: scalac signs the reflection trait's parameter `scala.quoted.Quotes.reflectModule.Printer`, which the
// signature match takes for the std's `scala.quoted.Reflect$.Printer` (the reflection table's pair), and a super
// accessor's signature selects the alternative of the member it calls (SuperAccessors.scala:77-81). With neither the
// call was taken for an ordinary one and the interpreter met an unresolved selection (eval.rs:183). Found by Codex's
// review of the workers exports helper's push 15.
package mcb

@main def run(): Unit =
  println(mck.Macros.Mac.printer)
  println(mck.Macros.Mac.sup)
