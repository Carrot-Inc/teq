package paths

object U5:
  val a = new Api(5)
  val b = new Api(1005)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
