package aoa

object O:
  object I:
    type T = Int

import O.I.T

object A:
  inline def inc(x: Int): Int = (x: T) + 1
  inline def incSel(x: Int): Int = (x: O.I.T) + 1
  def plain(x: Int): Int = (x: T) + 2
