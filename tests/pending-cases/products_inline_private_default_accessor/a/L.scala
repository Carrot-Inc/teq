package pdg

object O:
  private def pd(x: Int = 40): Int = x + 1
  inline def usePd: Int = pd() + pd(2)
