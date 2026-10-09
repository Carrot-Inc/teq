package views

class Label(val text: String):
  def shown: String = "label " + text

val title: String = "views"
def heading(text: String): String = "== " + text
