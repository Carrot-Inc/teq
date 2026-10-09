package retype

/** An interpolator's macro and the two others, in an object. */
object Panel:
  def title: String = "panel"
  def counted: Int = words"four five" + count("six")
  def shaped: String = shape("x yy zz")
