package paths

object U33:
  val a = new Api(33)
  val b = new Api(1033)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
