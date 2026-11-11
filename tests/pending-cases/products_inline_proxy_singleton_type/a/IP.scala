package ipa

trait Box:
  type T
  def get: T

class IntBox2 extends Box:
  type T = Int
  def get: Int = 7

object IP:
  transparent inline def s(b: Box): b.T = (b.get: b.T)
