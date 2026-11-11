package paths

object U40:
  val a = new Api(40)
  val b = new Api(1040)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
