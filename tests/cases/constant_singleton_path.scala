// A constant's selection is written as its literal and is still the path a singleton type names:
// `C.f` conforms to `C.f.type` for a `final val` with or without a literal type, as scalac folds
// only after typing.
object C:
  final val f = true
  final val g: 1 = 1
def cond(): C.f.type = { println("f"); C.f }
def one(): C.g.type = C.g
@main def run(): Unit =
  println(cond())
  println(one() + 1)
