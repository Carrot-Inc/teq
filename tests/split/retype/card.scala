package retype

/** A macro call in a method of a class and one in a top-level def. */
class Card(name: String):
  def text: String = "card " + name
  def counted: Int = count("one two three")

def cardShape: String = shape("a bb cc ddd")
