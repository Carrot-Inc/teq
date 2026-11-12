// A downstream of a/ that expands a/'s macro,
// whose implementation the interpreter is to
// run from a/'s products: the body is to apply
// `Pc(using q)(term)` as the using alternative,
// the one the pickle's signed name selects
// (`(scala.quoted.Quotes, java.lang.Object)`), as
// dotty's reader takes the clause from the method
// type (TreeUnpickler's `SELECTin`/`atSignature`).
package mub

@main def run(): Unit =
  println(muo.Mac.width)
