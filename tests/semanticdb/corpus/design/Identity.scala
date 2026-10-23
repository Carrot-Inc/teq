package identity
object Original:
  def f(x: Int): Int = x
object Forward:
  export Original.f
object Cash:
  val cash$ = Option(1)
  val use = cash$.get
  val result = Forward.f(1)
