package cia

// Casts in a module's products: the inline bodies' casts, a fixed class and the type parameter,
// pickled as written (`TypeApply(Select(e, asInstanceOf), T)`), are decided where a downstream
// expands them, and checked there whether the result is used or discarded; an ordinary method's
// cast is checked in its own body.
class A
class B

object Lib:
  inline def checked(x: Any): B = x.asInstanceOf[B]
  inline def generic[T](x: Any): T = x.asInstanceOf[T]
  def ordinary(x: Any): B = x.asInstanceOf[B]
