package ona

opaque type Id = Long
object Id:
  def apply(n: Long): Id = n
