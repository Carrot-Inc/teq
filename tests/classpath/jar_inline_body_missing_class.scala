// jars: scala-library missingdep-lib
// A jar's inline bodies that name a class of another library the program's class path does not hold
// (tests/support/missing_dep: `mdlib` compiled against `dep.Dep`, packaged without it). scalac 3.8.4 rejects each call
// whose expansion meets the class, `A.inl(1)` and `Fw.h2("x")(a = 5)`, whose omitted `b` takes the inline getter of
// `a` (the forwarder's getters numbered for its member): "undefined: new dep.Dep ... at inlining", the second with its
// inline stack trace; the call that writes every argument expands nothing that names it. teq's check of such a body
// is to fail on the class too. A non-retained inline method has no method at run time, so its call is to be an error
// at the call, the body's own diagnostics moved there with where they were inlined from, never a plain call
// (dotty reports an expansion's errors at the call, and `Erasure.checkNotErased` refuses any inline method's
// reference that survives, Erasure.scala:525-536). `B.d8`'s body is to fail with a default beside it: the
// method's error, not its default's alone.
// expect: jar_inline_body_missing_class.scala:21:11: error:
// expect: (in the body of mdlib.A.inl)
// expect: jar_inline_body_missing_class.scala:22:11: error:
// expect: inlined from
// expect: jar_inline_body_missing_class.scala:25:11: error:
// expect: (in the body of mdlib.B.d8)
// expect: 3 errors found
import mdlib.*
@main def run(): Unit =
  println(A.inl(1))
  println(Fw.h2("x")(a = 5))
  println(Fw.h2("x")(1, 2))
  println(A.plain(1))
  println(B.d8(1))
