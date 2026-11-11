// A downstream of a/, which runs a/'s bodies from its products in the interpreter and in JavaScript. A pickled
// application's clause is its function's: the method the pickle's signed name selects (`SELECTin`, TreeUnpickler's
// `SignedName` and `atSignature`), whose method type gives the clause (TypeAssigner's `Apply`). With two overloads of
// `apply`, one of a using clause, the reader took the clause from both and then from the argument's shape, so
// `v(using "V")(3)`, a literal no given, ran the plain overload: `plain:V3` over teq's and scalac's products, in the
// interpreter and in JavaScript, where scalac and teq's whole build print `using:V3`. Found by Codex's review of the
// workers exports helper's push 13. (A top-level method's overloads are another module's from its products under no
// name teq's runs find, products_toplevel_overload_member.)
package overload_use

import overload_using.Cases.*

@main def run(): Unit =
  println(Lib.literal)
  println(Lib.plain)
  println(Lib.givenArg)
  println(Lib.opsLiteral)
  println(Lib.opsPlain)
