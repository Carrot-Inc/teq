package ina

class Outer(val tag: String):
  class Inner(val n: Int):
    def show: String = tag + n
  object Helper:
    def make(n: Int): Inner = new Inner(n)
  def inner(n: Int): Inner = new Inner(n)

object Registry:
  class Entry(val key: String)
  object Defaults:
    val entry: Entry = new Entry("default")
