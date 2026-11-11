package paths

object U9:
  val a = new Api(9)
  val b = new Api(1009)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
