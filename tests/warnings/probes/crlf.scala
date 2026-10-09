object Lib { val a = 1; val b = 2 }
object Use {
  import Lib.{a,
    b}
  def f = a
}
