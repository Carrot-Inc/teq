package paths

object U13:
  val a = new Api(13)
  val b = new Api(1013)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
