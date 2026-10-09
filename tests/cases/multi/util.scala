package shapes.util

def banner(text: String): String = "== " + text + " =="

object Counter:
  private var n = 0
  def next(): Int =
    n += 1
    n
