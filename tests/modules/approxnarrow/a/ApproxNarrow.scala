package ana

// A dependent context function type's parent, whose argument under `Option` in the inner
// function's parameter is `Nothing`: the narrower function type takes it.
object ApproxNarrow:
  def widen[T](f: (s: String) ?=> List[Option[s.type]] => T): String ?=> List[Option[Nothing]] => T = f
