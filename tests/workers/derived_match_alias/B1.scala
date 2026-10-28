object B1:
  def pass[T <: Tuple](n: K.Count[T]): K.Count[T] = A.keep[T](n)
