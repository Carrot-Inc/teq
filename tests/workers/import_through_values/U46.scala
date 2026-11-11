package paths

object U46:
  val a = new Api(46)
  val b = new Api(1046)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
