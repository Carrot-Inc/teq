package paths

object U22:
  val a = new Api(22)
  val b = new Api(1022)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
