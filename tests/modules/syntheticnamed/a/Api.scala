package snm

// A polymorphic function type whose parameter the source names `x$1`, which a call names.
object Api:
  def keep(f: [A] => (x$1: A) => A) = f
