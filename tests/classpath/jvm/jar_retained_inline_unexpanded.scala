// jars: scala-library missingdep-lib
// A jar's retained inline method, one implementing a member that is not inline (tests/support/missing_dep's
// `Impl.tl`), has a method at run time: where teq's check cannot type its body (a named tuple's tail, its match types),
// the call is that method's, as dotty's refusal of an inline method's surviving reference spares a retained one
// (`isEffectivelyErased`, SymDenotations.scala:1076-1079), and the program prints scalac 3.8.4's output. Without its
// method, the call of a body teq cannot check is an error (jar_inline_body_missing_class).
import mdlib.*
@main def main(): Unit =
  val nt = (name = "n", age = 3)
  println(Impl.tl(nt))
