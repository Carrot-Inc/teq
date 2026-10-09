package oa

object Units:
  opaque type Meters = Double
  object Meters:
    def apply(d: Double): Meters = d
  extension (m: Meters)
    def value: Double = m
    def +(o: Meters): Meters = m + o
