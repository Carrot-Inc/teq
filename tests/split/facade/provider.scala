package prov

object Provider:
  def greet(name: String): String = s"hi $name"
  inline def twice(x: Int): Int = x * 2
  def label: String = "p"
