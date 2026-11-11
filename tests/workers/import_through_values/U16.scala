package paths

object U16:
  val a = new Api(16)
  val b = new Api(1016)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
