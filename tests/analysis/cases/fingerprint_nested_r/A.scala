package fpn

object A:
  object L:
    class C
  object R:
    class C
  inline def make: Any = new R.C
