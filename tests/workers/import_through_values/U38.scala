package paths

object U38:
  val a = new Api(38)
  val b = new Api(1038)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
