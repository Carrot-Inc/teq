package paths

object U42:
  val a = new Api(42)
  val b = new Api(1042)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
