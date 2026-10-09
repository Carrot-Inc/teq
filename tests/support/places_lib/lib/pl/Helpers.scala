package pl

object Helpers:
  inline def removedOf(b: PlBox): Int = b.removed
  transparent inline def sharedRemoved: Int = PlBox.of(1).removed
