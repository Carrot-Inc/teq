package tma

abstract class Codec:
  type T
  type Wire <: CharSequence
  def encode(t: T): Wire
  def decode(w: Wire): T

trait Registry:
  type Key = String
  type Value
  def put(k: Key, v: Value): Unit
