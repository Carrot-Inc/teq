package probe
class C:
  protected var x = 0
  private[probe] var y = 0
  inline def get: Int = x + y
  inline def set(v: Int): Unit = { x = v; y = v }
final class F:
  private var z = 0
  inline def get: Int = z
  inline def set(v: Int): Unit = z = v
object O:
  private var w = 0
  inline def get: Int = w
  inline def set(v: Int): Unit = w = v
