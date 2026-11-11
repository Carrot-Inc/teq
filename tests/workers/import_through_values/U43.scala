package paths

object U43:
  val a = new Api(43)
  val b = new Api(1043)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
