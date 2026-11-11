package paths

object U37:
  val a = new Api(37)
  val b = new Api(1037)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
