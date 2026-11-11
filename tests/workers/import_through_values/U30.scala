package paths

object U30:
  val a = new Api(30)
  val b = new Api(1030)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
