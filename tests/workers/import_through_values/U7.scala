package paths

object U7:
  val a = new Api(7)
  val b = new Api(1007)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
