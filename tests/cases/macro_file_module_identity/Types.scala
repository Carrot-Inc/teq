package own
opaque type Id = String
object Id:
  def apply(s: String): Id = s
