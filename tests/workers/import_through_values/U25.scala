package paths

object U25:
  val a = new Api(25)
  val b = new Api(1025)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
