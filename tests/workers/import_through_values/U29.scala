package paths

object U29:
  val a = new Api(29)
  val b = new Api(1029)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
