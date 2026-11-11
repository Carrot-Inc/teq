package cfa

object Lib:
  def widen[T](f: (s: String) ?=> List[Option[s.type]] => T): String ?=> List[Option[Nothing]] => T = f
