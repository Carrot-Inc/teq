package shared

opaque type Id = Int

object Id:
  private[shared] def authorize(value: Int): Id = value

  extension (id: Id)
    def value: Int = id
