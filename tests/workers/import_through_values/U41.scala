package paths

object U41:
  val a = new Api(41)
  val b = new Api(1041)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
