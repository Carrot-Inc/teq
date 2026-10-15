// `C.apply` of a case class nested in a class, given to a value of a function type, is eta-expanded to
// that type before any construction through the companion's prefix (`Typer.adapt`'s method reference).
class Outer:
  case class C(x: Int, y: String)
  val make: (Int, String) => C = C.apply
  def test(): Unit = println(make(1, "x"))
@main def run(): Unit = new Outer().test()
