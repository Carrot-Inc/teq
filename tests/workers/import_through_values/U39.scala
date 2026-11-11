package paths

object U39:
  val a = new Api(39)
  val b = new Api(1039)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
