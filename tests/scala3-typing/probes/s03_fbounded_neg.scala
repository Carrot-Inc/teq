trait Ord[T <: Ord[T]]
def f[T <: Ord[T]](t: T): T = t
def test = f(1)
@main def run(): Unit = println(1)
