object P:
  val x = "P"
object T:
  val x = "T"
  def f =
    import P.x
    x
  def g(x: String) =
    import P.x
    x
@main def run(): Unit =
  println(T.f)
  println(T.g("param"))
