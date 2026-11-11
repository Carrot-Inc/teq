package paths

object U45:
  val a = new Api(45)
  val b = new Api(1045)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
