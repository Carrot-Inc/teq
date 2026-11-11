package tta

class Outer:
  type T = Int
  inline def f(x: Int): Int = (x: this.T)
  def g(x: Int): Int = (x: this.T) + 0
