object A:
  def keep[T <: Tuple](n: K.Count[T]): K.Count[T] = n
  inline def via[T <: Tuple](inline n: K.Count[T]): K.Count[T] = scala.compiletime.summonFrom { case _ => (n: K.Count[T]) }
