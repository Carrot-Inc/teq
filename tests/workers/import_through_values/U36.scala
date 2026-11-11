package paths

object U36:
  val a = new Api(36)
  val b = new Api(1036)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
