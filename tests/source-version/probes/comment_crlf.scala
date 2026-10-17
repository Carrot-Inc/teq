object P {
  def f(implicit x: Int): Int = x
  val y = f( // a note
    1)
}
