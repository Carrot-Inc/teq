package oca

opaque type ShelfKey = Int
object ShelfKey:
  def of(value: Int): ShelfKey = value
  extension (k: ShelfKey)
    def value: Int = k
