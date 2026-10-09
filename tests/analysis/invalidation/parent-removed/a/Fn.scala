package pra

trait Marker:
  def mark: Int = 0

abstract class Fn extends Marker:
  def apply(x: Int): Int
