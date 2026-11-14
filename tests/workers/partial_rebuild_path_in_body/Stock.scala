package rebuilt

class Base(val name: String):
  val tag = { val n = name.length; n + 1 }

object Stock:
  val made = new Base("b" + 1)
