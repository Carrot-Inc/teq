// teq: --no-outline
object Comparisons:
  inline def same(inline left: AnyRef)(inline right: AnyRef): Boolean = left eq right
  inline def different(inline left: AnyRef)(inline right: AnyRef): Boolean = !same(left)(right)

@main def probe(): Unit =
  val stable = new Object
  var evaluated = 0
  def value(): AnyRef =
    evaluated += 1
    stable
  val direct = Comparisons.different(stable)(null)
  val computed = Comparisons.different(value())(null)
  println(s"$direct $computed $evaluated")
