package namesprobe
class `User$anonClass`:
  private var x = 0
  inline def get: Int = x
  inline def set(v: Int): Unit = x = v
object Factory:
  def make: Any =
    class Local:
      private val x = 1
      inline def get: Int = x
    new Local
