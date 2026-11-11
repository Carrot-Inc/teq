package object holders:
  def decorate(value: String): String = "[" + value + "]"
  given flag: Boolean = true
