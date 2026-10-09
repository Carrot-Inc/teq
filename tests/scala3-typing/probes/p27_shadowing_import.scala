object P:
  val x = 10
object T:
  val x = 1
  def f =
    import P.x
    x
  def g(x: Int) =
    import P.x
    x
@main def run(): Unit = println(T.f)
