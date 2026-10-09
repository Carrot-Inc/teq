object Wrap:
  inline def failWrapped: Unit = Pos.fail(true)
  inline def failPlain: Unit = Pos.fail(false)
