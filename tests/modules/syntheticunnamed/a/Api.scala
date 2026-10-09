package snu

// A polymorphic function type of an unnamed parameter, which scalac's `apply` names `x$1`
// (dotty's `Desugar.makePolyFunctionType`), and a call names.
object Api:
  def keep(f: [A] => A => A) = f
