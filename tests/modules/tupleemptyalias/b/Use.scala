// A downstream of a/, whose `*:` chains end in a/'s own alias `custom.Types.EmptyTuple` (`Tuple1[String]`). The reader
// keeps a chain's tail as its reference denotes it (`TreeUnpickler.readType`'s `TypeRef(pre, name)`): only
// `scala.Tuple$package.EmptyTuple` is the std's empty tuple alias, which `TypeErasure.tupleArity` reads as no arity.
// Before, the reader took any tail named `EmptyTuple` for the std's, and `Lib.take((1, "yes"))` was a type mismatch
// (found `(Int, String)`, required `*:[Int, EmptyTuple]`) over scalac's products and teq's alike, where scalac and
// integration 2d256830e9 print the run.txt. Found by Codex's review of the workers exports helper's push 11.
package tub

import custom.Lib

@main def run(): Unit =
  println(Lib.take((1, "yes")))
  println(Lib.make.tail.head)
