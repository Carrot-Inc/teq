package r3a
object Api:
  def kind(o: Any): String = o match
    case _: Array[?] => "A"
    case _ => "?"
