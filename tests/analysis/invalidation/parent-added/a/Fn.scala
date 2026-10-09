package paa

trait Marker:
  def mark: Int = 0

abstract class Fn:
  def apply(x: Int): Int
