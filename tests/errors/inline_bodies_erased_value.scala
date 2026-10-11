// jars: inlinebodies-lib
// command: build
// expect: 8:33: error: method erasedValue is declared as `erased`, but is in fact used
// expect: 1 error found
// An `erasedValue` the expansion of a jar's inline body keeps as a value (tests/support/inline_bodies_lib.scala's
// `zero`) is rejected as scalac's `Erasure.checkNotErased` rejects it, the body expanded from its pickle.
import inlinebodies.Bodies
@main def run(): Unit = println(Bodies.zero[Int])
