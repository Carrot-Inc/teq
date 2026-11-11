package paths

object U35:
  val a = new Api(35)
  val b = new Api(1035)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
