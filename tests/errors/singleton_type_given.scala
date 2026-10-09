// expect: 9:39: error: no given instance of type Show[(Red : Color)] was found for parameter x
// expect: 1 error found
// `Color.Red.type` is the singleton type of the case, which a `Show[Color]` does not implement.
enum Color:
  case Red, Green
trait Show[A] { def show(a: A): String }
given Show[Color] = c => "color " + c
@main def Main(): Unit =
  println(summon[Show[Color.Red.type]].show(Color.Red))
