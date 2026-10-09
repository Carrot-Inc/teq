package qa

// Qualified access names its class by its full name.
class Outer:
  private[Outer] def f: Int = 1
  protected[Outer] def g: Int = 2
  class Inner:
    private[Outer] def h: Int = 3
    private[Inner] def i: Int = 4

object Obj:
  private[Obj] def j: Int = 5
  class In:
    private[Obj] def k: Int = 6
    private[In] def l: Int = 7

class Top:
  private[qa] def m: Int = 8
