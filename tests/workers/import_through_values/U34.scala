package paths

object U34:
  val a = new Api(34)
  val b = new Api(1034)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
