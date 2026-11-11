package paths

object U32:
  val a = new Api(32)
  val b = new Api(1032)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
