package kept

class Box[T <: String]

val x = 1

/** A bound of a signature written out that hangs on a type left to inference. */
def f(a: Box[x.type]): Int = 0
