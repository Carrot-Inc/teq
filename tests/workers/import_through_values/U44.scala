package paths

object U44:
  val a = new Api(44)
  val b = new Api(1044)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
