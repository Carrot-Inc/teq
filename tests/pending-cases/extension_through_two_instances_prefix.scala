// An extension method imported from a path whose
// type is two instances of its owner (`x: IB & SB`,
// `import x.ext`): `ext(a: A): A` seen from `x` takes
// `Int | String` and gives `Int & String` (scalac 3.8.4).
trait Box[A]:
  extension (a: A) def ext: A = a
trait IB extends Box[Int]
trait SB extends Box[String]
def u(x: IB & SB, v: Int & String): Int & String =
  import x.ext
  v.ext

@main def run(): Unit = println("ok")
