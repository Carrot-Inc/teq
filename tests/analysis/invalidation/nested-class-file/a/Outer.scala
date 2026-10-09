package nca

class Outer:
  class Inner:
    def v: Int = 1
  def make: Inner = new Inner
