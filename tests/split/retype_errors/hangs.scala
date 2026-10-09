package errs

class Crate[T <: String]

val width = 1

/** A bound of a signature that hangs on a type left to inference, in a program whose bodies
  * have errors elsewhere. */
def hung(a: Crate[width.type]): Int = 0
