package fix.casts

// Casts in a library's bodies, for a program built over its TASTy (tests/classpath/js and jvm,
// scala_product_casts): the inline bodies' casts, a fixed class and the type parameter, are
// decided where the program expands them; an ordinary method's is its own.
class CastA
class CastB

object CastLib:
  inline def checked(x: Any): CastB = x.asInstanceOf[CastB]
  inline def generic[T](x: Any): T = x.asInstanceOf[T]
  def ordinary(x: Any): CastB = x.asInstanceOf[CastB]
