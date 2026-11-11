package paths

object U31:
  val a = new Api(31)
  val b = new Api(1031)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
