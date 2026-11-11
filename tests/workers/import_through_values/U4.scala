package paths

object U4:
  val a = new Api(4)
  val b = new Api(1004)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
