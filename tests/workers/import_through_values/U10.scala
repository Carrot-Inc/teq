package paths

object U10:
  val a = new Api(10)
  val b = new Api(1010)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
