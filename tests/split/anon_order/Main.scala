// `val a, b = new T` types the initialiser once per name, and the class each copy makes is
// numbered by that order; the given typed `b.type` makes the signature phase's tables ask for
// `b` before the walk reaches `a`, so the copies have to be typed in their order whichever is
// asked for first (a retype, which walks the file in order, must agree with a fresh build).
trait T:
  def value: Int
  def name: String
object C:
  val a, b = new T:
    def value: Int = 1
    def name: String = "t"
  given selected: b.type = b
@main def main(): Unit =
  println(C.a.value + C.b.value)
  println(C.a.name + C.b.name)
