package np.sub

object O:
  type T = Int

import O.T

object Pk:
  inline def n7(x: Int): Int = compiletime.summonInline[Ordering[T]].compare(x, 1)
  inline def n8(x: Int): Int = compiletime.summonInline[Ordering[Int]].compare(x, 1)
