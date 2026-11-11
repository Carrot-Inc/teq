package paths

object U24:
  val a = new Api(24)
  val b = new Api(1024)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
