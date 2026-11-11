package paths

object U21:
  val a = new Api(21)
  val b = new Api(1021)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
