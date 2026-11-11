package tga

trait T:
  def v: Int

given G2: T with
  def v: Int = 3

given x3: Int = 1
