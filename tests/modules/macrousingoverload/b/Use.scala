// A downstream of a/ that expands a/'s macro, whose implementation the interpreter runs from a/'s products: the
// converted body applies `Pc(using q)(term)` as the using alternative, the one the pickle's signed name selects
// (`(scala.quoted.Quotes, java.lang.Object)`), as dotty's reader takes the clause from the method type
// (TreeUnpickler's `SELECTin`/`atSignature`). The reader matched the signature against teq's erasure of the
// std's `Reflect.Term` class where scala-library's abstract type erases to Object, found no alternative, and since
// push 14 took the application for an ordinary one: the interpreter met an unresolved selection ("resolved
// selection", interp/eval.rs:183), zio-test's `assertTrue` path in the api's test check. Likewise scala-library's
// `sorted`, whose super accessor no class of teq's holds.
package mub

@main def run(): Unit =
  println(muo.Mac.width)
