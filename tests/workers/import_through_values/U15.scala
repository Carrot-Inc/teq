package paths

object U15:
  val a = new Api(15)
  val b = new Api(1015)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
