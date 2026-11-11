package paths

object U18:
  val a = new Api(18)
  val b = new Api(1018)
  def f: Int =
    import a.R.*
    value
  def g: Int =
    import b.R.*
    value
  def h: Int = f * 10000 + g
