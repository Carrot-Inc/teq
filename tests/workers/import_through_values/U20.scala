package paths

object U20:
  val a = new Api(20)
  val b = new Api(1020)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
