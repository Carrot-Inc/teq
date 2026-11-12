// A downstream of a/, which is to run a/'s bodies from its products in the interpreter
// and in JavaScript. A pickled application's clause is its function's:
// the method the pickle's signed name selects (`SELECTin`, TreeUnpickler's
// `SignedName` and `atSignature`), whose method type gives the clause
// (TypeAssigner's `Apply`). With two overloads of `apply`, one of a using clause,
// `v(using "V")(3)`, a literal no given, is to print `using:V3` over teq's and
// scalac's products, in the interpreter and in JavaScript, as scalac prints
// it. (A top-level method's overloads: products_toplevel_overload_member.)
package overload_use

import overload_using.Cases.*

@main def run(): Unit =
  println(Lib.literal)
  println(Lib.plain)
  println(Lib.givenArg)
  println(Lib.opsLiteral)
  println(Lib.opsPlain)
