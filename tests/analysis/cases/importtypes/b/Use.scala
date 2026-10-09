package itb

import ita.Box

// Imports nothing uses of a value's type member and of an object's exported type and term.
class Use:
  def f(box: Box): Int =
    import box.T
    0
  def g: Int =
    import ita.B.U
    0
  def h: Int =
    import ita.B.n
    0
