package scala2lib.indirect

trait Left { def left(text: String): String = "left " + text }
trait Right { def right(text: String): String = "right " + text }
trait Both extends Left with Right
