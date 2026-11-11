package scala2lib.overloaded

trait Entries {
  def render(value: String): String = value
  def render(value: Int): String = value.toString
}
