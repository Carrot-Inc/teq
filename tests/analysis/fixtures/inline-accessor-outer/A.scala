package nest
class Outer:
  private var x = 0
  class Inner:
    inline def get: Int = x
    inline def set(v: Int): Unit = x = v
