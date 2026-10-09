package kept.two

class Box[T <: String]

val x = 1

/** The same with the class's import error in another file. */
def f(a: Box[x.type]): Int = 0
