package paths

object U2:
  val a = new Api(2)
  val b = new Api(1002)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
