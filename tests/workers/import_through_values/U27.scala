package paths

object U27:
  val a = new Api(27)
  val b = new Api(1027)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
