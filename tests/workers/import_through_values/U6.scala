package paths

object U6:
  val a = new Api(6)
  val b = new Api(1006)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
