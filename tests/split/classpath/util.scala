package util

object Fmt {
  def show(xs: List[Int]): String = xs.mkString("[", ",", "]")
}
