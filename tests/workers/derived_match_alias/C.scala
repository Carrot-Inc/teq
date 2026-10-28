object C:
  def use[T <: Tuple](n: K.Count[T]): K.Count[T] = A.via[T](n)
